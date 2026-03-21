set dotenv-load := false
# Use a Windows-friendly shell if available; defaults remain for *nix.
# Powershell is present on Windows by default.
set windows-shell := ["powershell.exe", "-NoLogo", "-NonInteractive", "-Command"]

# Governance kernel root: when HANDSHAKE_GOV_ROOT is set, all governance
# scripts/checks/protocols resolve from that external worktree.
# Falls back to the repo-local .GOV/ when unset (backwards-compatible).
GOV_ROOT := env_var_or_default('HANDSHAKE_GOV_ROOT', '.GOV')

# External build/test artifacts (Cargo target dir) MUST live outside the repo working tree.
CARGO_TARGET_DIR := "../Handshake Artifacts/handshake-cargo-target"

dev: preflight-ollama
	node -e "const {execFileSync}=require('child_process'); const path=require('path'); const repo=execFileSync('git',['rev-parse','--show-toplevel'],{encoding:'utf8'}).trim(); const cargoTarget=path.resolve(repo,'{{CARGO_TARGET_DIR}}'); execFileSync('pnpm',['-C','app','run','tauri','dev'],{stdio:'inherit', env:{...process.env, CARGO_TARGET_DIR:cargoTarget}});"

# Fail fast if Ollama is missing/unreachable (Phase 1 requirement; see {{GOV_ROOT}}/roles_shared/docs/START_HERE.md).
preflight-ollama:
	node -e "const base=(process.env.OLLAMA_URL||'http://localhost:11434'); const normalized=base.endsWith('/')?base.slice(0,-1):base; const url=normalized + '/api/tags'; const lib=url.startsWith('https://')?require('https'):require('http'); const req=lib.get(url,(res)=>{ const ok=!!res.statusCode && res.statusCode>=200 && res.statusCode<300; if(ok){ process.exit(0); } console.error('Ollama preflight failed: GET ' + url + ' returned ' + res.statusCode + '. Install Ollama using your platform package manager or installer (for example `winget install -e --id Ollama.Ollama` on Windows), then run ollama serve (or ollama run mistral), or set OLLAMA_URL.'); process.exit(1); }); req.on('error',()=>{ console.error('Ollama preflight failed: cannot reach ' + url + '. Install Ollama using your platform package manager or installer (for example `winget install -e --id Ollama.Ollama` on Windows), then run ollama serve (or ollama run mistral), or set OLLAMA_URL.'); process.exit(1); }); req.setTimeout(3000, ()=>req.destroy(new Error('timeout')));"

lint:
	cd app; pnpm run lint
	cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features --target-dir "{{CARGO_TARGET_DIR}}"

test:
	cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "{{CARGO_TARGET_DIR}}"

# Fail if any required docs are missing (navigation pack + shared tooling guardrails + resilience)
docs-check:
	node -e "['{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md', '{{GOV_ROOT}}/README.md', '{{GOV_ROOT}}/roles/README.md', '{{GOV_ROOT}}/roles_shared/README.md', '{{GOV_ROOT}}/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md', '{{GOV_ROOT}}/roles/coder/CODER_PROTOCOL.md', '{{GOV_ROOT}}/roles/validator/VALIDATOR_PROTOCOL.md', '{{GOV_ROOT}}/roles_shared/docs/START_HERE.md', '{{GOV_ROOT}}/spec/SPEC_CURRENT.md', '{{GOV_ROOT}}/roles_shared/docs/ARCHITECTURE.md', '{{GOV_ROOT}}/roles_shared/docs/RUNBOOK_DEBUG.md', '{{GOV_ROOT}}/roles_shared/docs/REPO_RESILIENCE.md', '{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md', '{{GOV_ROOT}}/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md', '{{GOV_ROOT}}/docs/vscode-session-bridge/GOVERNED_SESSION_CONTROL_ARCHITECTURE.md'].forEach(f => { if (!require('fs').existsSync(f)) { console.error('Missing: ' + f); process.exit(1); } })"

# Format backend Rust
fmt:
	cd src/backend/handshake_core; cargo fmt

# Clean Cargo artifacts in the external target dir ({{CARGO_TARGET_DIR}})
cargo-clean:
	cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "{{CARGO_TARGET_DIR}}"

# Full hygiene pass: docs, lint, tests, fmt, clippy
validate:
	just docs-check
	just codex-check
	just scaffold-check
	just codex-check-test
	cd app; pnpm run lint
	cd app; pnpm test
	cd app; pnpm run depcruise
	cd src/backend/handshake_core; cargo fmt
	cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features --target-dir "{{CARGO_TARGET_DIR}}"
	cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "{{CARGO_TARGET_DIR}}"
	cargo deny --manifest-path src/backend/handshake_core/Cargo.toml check advisories licenses bans sources

# Codex guardrails: prevent direct fetch in components, println/eprintln in backend, and doc drift.
codex-check:
	node {{GOV_ROOT}}/roles_shared/checks/codex-check.mjs

# Governance-only checks (drive-agnostic + lifecycle UX + task board integrity).
gov-check:
	just docs-check
	node {{GOV_ROOT}}/roles_shared/checks/gov-check.mjs

gov-shared-tests:
	node --test {{GOV_ROOT}}/roles_shared/tests/*.test.mjs

# Run gov-check and auto-route failures as notifications to the responsible WP coder(s)
gov-check-feedback wp-id='' session='orchestrator':
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/gov-check-feedback.mjs {{wp-id}} --session={{session}}

protocol-alignment-check:
	node {{GOV_ROOT}}/roles_shared/checks/protocol-alignment-check.mjs

governance-structure-audit:
	node {{GOV_ROOT}}/roles_shared/checks/governance-structure-check.mjs

governance-structure-check:
	node {{GOV_ROOT}}/roles_shared/checks/governance-structure-check.mjs --strict

governance-map:
	@Get-Content {{GOV_ROOT}}/README.md

role-bundle role:
	@Get-Content {{GOV_ROOT}}/roles/{{role}}/README.md

validation-map:
	@Get-Content {{GOV_ROOT}}/roles_shared/checks/README.md

semantic-proof-check:
	node {{GOV_ROOT}}/roles_shared/checks/semantic-proof-check.mjs

topology-registry-sync:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/topology-registry-sync.mjs

topology-registry-check:
	node {{GOV_ROOT}}/roles_shared/checks/topology-registry-check.mjs

# Safety backup push: push the current committed branch state to its matching GitHub backup branch.
backup-push local_branch="" remote_branch="":
	node {{GOV_ROOT}}/roles_shared/scripts/topology/backup-push.mjs {{local_branch}} {{remote_branch}}

# Ensure the permanent GitHub backup branches exist, seeded from local main when missing.
ensure-permanent-backup-branches:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/ensure-permanent-backup-branches.mjs

# Immutable out-of-repo snapshot: git bundles + copied working files.
backup-snapshot label="manual" out_root="" nas_root="":
	node {{GOV_ROOT}}/roles_shared/scripts/topology/backup-snapshot.mjs --label "{{label}}" --out-root "{{out_root}}" --nas-root "{{nas_root}}"

# Read-only status for local/NAS backup roots and latest snapshot presence.
backup-status:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/backup-status.mjs

# Immutable snapshot using the configured HANDSHAKE_NAS_BACKUP_ROOT.
backup-snapshot-nas label="manual":
	node {{GOV_ROOT}}/roles_shared/scripts/topology/backup-snapshot.mjs --label "{{label}}" --require-nas

# Fast-forward the permanent local clones from their matching remotes when all are clean.
sync-all-role-worktrees:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/sync-all-role-worktrees.mjs

# Copy governance kernel .GOV/ into the main worktree and auto-commit.
# RESPONSIBILITY: Integration Validator by default; Orchestrator may run it under explicit Operator instruction [CX-212D].
sync-gov-to-main:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/sync-gov-to-main.mjs

# Enumerate deletable local worktrees/branches and non-protected remote branches with exact approval examples.
enumerate-cleanup-targets:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/enumerate-cleanup-targets.mjs

# Delete a non-protected git-managed local worktree safely: immutable snapshot first, then git worktree remove only.
delete-local-worktree worktree_id approval:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/delete-local-worktree.mjs {{worktree_id}} --approve "{{approval}}"

# Delete a non-protected git-managed local worktree using an already-created immutable snapshot root.
delete-local-worktree-precreated worktree_id approval snapshot_root:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/delete-local-worktree.mjs {{worktree_id}} --approve "{{approval}}" --precreated-snapshot-root "{{snapshot_root}}"

# Delete a non-protected git-managed local worktree using an already-created immutable snapshot root and a safety stash for dirty state.
delete-local-worktree-precreated-stash worktree_id approval snapshot_root:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/delete-local-worktree.mjs {{worktree_id}} --approve "{{approval}}" --precreated-snapshot-root "{{snapshot_root}}" --stash-dirty

# Generate a single-target, token-gated cleanup script for a merged WP role worktree.
generate-worktree-cleanup-script wp-id role:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/generate-worktree-cleanup-script.mjs {{wp-id}} {{role}}

# Master Spec EOF appendix blocks check (Spec §12).
spec-eof-appendices-check:
	node {{GOV_ROOT}}/roles_shared/checks/spec-eof-appendices-check.mjs

# Governance sync helper: refresh derived governance views then validate.
gov-sync:
	just build-order-sync
	just topology-registry-sync
	just gov-check

spec-debt-open wp-id clause notes blocking="NO":
	node {{GOV_ROOT}}/roles_shared/scripts/debt/spec-debt-open.mjs {{wp-id}} "{{clause}}" "{{notes}}" {{blocking}}

spec-debt-close debt-id:
	node {{GOV_ROOT}}/roles_shared/scripts/debt/spec-debt-close.mjs {{debt-id}}

spec-debt-sync wp-id:
	node {{GOV_ROOT}}/roles_shared/scripts/debt/spec-debt-sync.mjs {{wp-id}}

# Build order (derived view) maintenance [CX-BO-001]
build-order-sync:
	node {{GOV_ROOT}}/roles_shared/scripts/build-order-sync.mjs

build-order-check:
	node {{GOV_ROOT}}/roles_shared/checks/build-order-check.mjs

# Worktrees (recommended when >1 WP active)
# Creates a dedicated working directory for the WP branch.
worktree-add wp-id base="main" branch="" dir="":
	node {{GOV_ROOT}}/roles_shared/scripts/topology/worktree-add.mjs {{wp-id}} {{base}} {{branch}} {{dir}}

# Role-scoped WP worktrees for ORCHESTRATOR_MANAGED CLI sessions.
coder-worktree-add wp-id branch="" dir="":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/role-session-worktree-add.mjs CODER {{wp-id}} {{branch}} {{dir}}

# [CX-212D] WP Validator uses the coder worktree — no-op, prints guidance.
wp-validator-worktree-add wp-id branch="" dir="":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/role-session-worktree-add.mjs WP_VALIDATOR {{wp-id}} {{branch}} {{dir}}

# [CX-212D] Integration Validator uses handshake_main — no-op, prints guidance.
integration-validator-worktree-add wp-id branch="" dir="":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/role-session-worktree-add.mjs INTEGRATION_VALIDATOR {{wp-id}} {{branch}} {{dir}}

# Repo-governed session launch helpers.
# AUTO = Orchestrator queues a VS Code plugin launch first; CLI fallback is unlocked only after 2 plugin failures/timeouts.
launch-coder-session wp-id host="AUTO" model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs CODER {{wp-id}} {{host}} {{model}}

launch-wp-validator-session wp-id host="AUTO" model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs WP_VALIDATOR {{wp-id}} {{host}} {{model}}

launch-integration-validator-session wp-id host="AUTO" model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs INTEGRATION_VALIDATOR {{wp-id}} {{host}} {{model}}

session-registry-status wp-id="":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-registry-status.mjs {{wp-id}}

handshake-acp-bridge:
	node {{GOV_ROOT}}/tools/handshake-acp-bridge/agent.mjs

session-start role wp-id model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs START_SESSION {{role}} {{wp-id}} "" {{model}}

session-send role wp-id prompt model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs SEND_PROMPT {{role}} {{wp-id}} "{{prompt}}" {{model}}

session-cancel role wp-id:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-cancel.mjs {{role}} {{wp-id}}

session-close role wp-id:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs CLOSE_SESSION {{role}} {{wp-id}}

handshake-acp-broker-status:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-broker.mjs status

handshake-acp-broker-stop:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-broker.mjs stop

start-coder-session wp-id model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs START_SESSION CODER {{wp-id}} "" {{model}}

start-wp-validator-session wp-id model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs START_SESSION WP_VALIDATOR {{wp-id}} "" {{model}}

start-integration-validator-session wp-id model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs START_SESSION INTEGRATION_VALIDATOR {{wp-id}} "" {{model}}

steer-coder-session wp-id prompt model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs SEND_PROMPT CODER {{wp-id}} "{{prompt}}" {{model}}

cancel-coder-session wp-id:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-cancel.mjs CODER {{wp-id}}

close-coder-session wp-id:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs CLOSE_SESSION CODER {{wp-id}}

steer-wp-validator-session wp-id prompt model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs SEND_PROMPT WP_VALIDATOR {{wp-id}} "{{prompt}}" {{model}}

cancel-wp-validator-session wp-id:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-cancel.mjs WP_VALIDATOR {{wp-id}}

close-wp-validator-session wp-id:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs CLOSE_SESSION WP_VALIDATOR {{wp-id}}

steer-integration-validator-session wp-id prompt model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs SEND_PROMPT INTEGRATION_VALIDATOR {{wp-id}} "{{prompt}}" {{model}}

cancel-integration-validator-session wp-id:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-cancel.mjs INTEGRATION_VALIDATOR {{wp-id}}

close-integration-validator-session wp-id:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs CLOSE_SESSION INTEGRATION_VALIDATOR {{wp-id}}

session-launch-runtime-check:
	node {{GOV_ROOT}}/roles_shared/checks/session-launch-runtime-check.mjs

# Hard gate helper: Worktree + Branch Gate [CX-WT-001]
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

# Protocol ack helper: print first non-empty line from each required doc.
protocol-ack codex agents shared protocol:
	@node {{GOV_ROOT}}/roles_shared/scripts/protocol-ack.mjs "{{codex}}" "{{agents}}" "{{shared}}" "{{protocol}}"

task-board-check:
	node {{GOV_ROOT}}/roles_shared/checks/task-board-check.mjs

task-packet-claim-check:
	node {{GOV_ROOT}}/roles_shared/checks/task-packet-claim-check.mjs

phase1-add-coverage-check:
	node {{GOV_ROOT}}/roles_shared/checks/phase1-add-coverage-check.mjs

# Dependency cruise (frontend architecture)
depcruise:
	cd app; pnpm run depcruise

# Dependency & license checks (Rust)
deny:
	cargo deny --manifest-path src/backend/handshake_core/Cargo.toml check advisories licenses bans sources

# Scaffolding
new-react-component name:
	node {{GOV_ROOT}}/roles_shared/scripts/dev/new-react-component.mjs {{name}}

new-api-endpoint name:
	node {{GOV_ROOT}}/roles_shared/scripts/dev/new-api-endpoint.mjs {{name}}

scaffold-check:
	node {{GOV_ROOT}}/roles_shared/scripts/dev/scaffold-check.mjs

codex-check-test:
	node {{GOV_ROOT}}/roles_shared/scripts/dev/codex-check-test.mjs

# Close a WP branch after it has been merged into main.
close-wp-branch wp-id remote="" approval="":
	node {{GOV_ROOT}}/roles_shared/scripts/topology/close-wp-branch.mjs {{wp-id}} {{remote}} --approve "{{approval}}"

# === Workflow Enforcement Commands (see {{GOV_ROOT}}/spec/SPEC_CURRENT.md) ===

# Orchestrator preflight (condensed): worktree context + governance integrity + spec regression.
orchestrator-preflight:
	@just hard-gate-wt-001
	@just gov-check
	@just validator-spec-regression

# Validator preflight (condensed): worktree context + governance integrity + spec regression.
validator-preflight:
	@just hard-gate-wt-001
	@just gov-check
	@just validator-spec-regression

# Coder preflight (condensed): worktree context + governance integrity + spec regression.
coder-preflight:
	@just hard-gate-wt-001
	@just gov-check
	@just validator-spec-regression

# Role startup (recommended): protocol ack + condensed preflight in one command.
orchestrator-startup:
	@just protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md"
	@just backup-status
	@just orchestrator-preflight
	@echo 'RESUME_HINT: After a reset/compaction, run `just orchestrator-next [WP-{ID}]` and continue automatically when OPERATOR_ACTION: NONE.'

validator-startup:
	@just protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/validator/VALIDATOR_PROTOCOL.md"
	@just backup-status
	@just validator-preflight
	@echo 'RESUME_HINT: After a reset/compaction, run `just validator-next [WP-{ID}]` and continue automatically when OPERATOR_ACTION: NONE.'

coder-startup:
	@just protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/coder/CODER_PROTOCOL.md"
	@just backup-status
	@just coder-preflight
	@echo 'RUBRIC_REQUIRED: Read `{{GOV_ROOT}}/roles/coder/docs/CODER_RUBRIC_V2.md` before the first WP-specific BOOTSTRAP or code change, and answer it in `## STATUS_HANDOFF` before validator handoff.'
	@echo 'RESUME_HINT: After a reset/compaction, run `just coder-next [WP-{ID}]` and continue automatically when OPERATOR_ACTION: NONE.'

# Record a technical refinement for a work packet [CX-585A]
record-refinement wp-id detail="":
	@node {{GOV_ROOT}}/roles/orchestrator/checks/orchestrator_gates.mjs refine {{wp-id}} "{{detail}}"

# Record a user signature bundle for a work packet [CX-585C]
# Current workflow requires: workflow lane + execution owner.
# Legacy recovery still accepts the older single execution-lane form.
# Allowed workflow lanes: MANUAL_RELAY | ORCHESTRATOR_MANAGED
# Allowed execution owners for current runs: Coder-A .. Coder-Z
record-signature wp-id signature workflow_lane="" execution_lane="":
	@node {{GOV_ROOT}}/roles/orchestrator/checks/orchestrator_gates.mjs sign {{wp-id}} {{signature}} {{workflow_lane}} {{execution_lane}}

# Record WP preparation (branch/worktree + execution owner) after signature and before packet creation.
# If omitted, workflow lane / execution owner are inferred from the signed bundle.
record-prepare wp-id workflow_lane="" execution_lane="" branch="" worktree_dir="":
	@node {{GOV_ROOT}}/roles/orchestrator/checks/orchestrator_gates.mjs prepare {{wp-id}} {{workflow_lane}} {{execution_lane}} {{branch}} {{worktree_dir}}

# Orchestrator helper (read-only): infer next steps for a WP from gates + file state.
orchestrator-next wp-id="":
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/orchestrator-next.mjs {{wp-id}}

# Coder helper (read-only): infer next steps for the current WP after reset/compaction.
coder-next wp-id="":
	@node {{GOV_ROOT}}/roles/coder/scripts/coder-next.mjs {{wp-id}}

# Validator helper (read-only): infer next steps for the current WP after reset/compaction.
validator-next wp-id="":
	@node {{GOV_ROOT}}/roles/validator/scripts/validator-next.mjs {{wp-id}}

# Deterministic Task Board updater: move a WP entry between sections.
task-board-set wp-id status reason="":
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/task-board-set.mjs {{wp-id}} {{status}} "{{reason}}"

# Deterministic traceability mapping updater: set Base WP -> Active Packet.
wp-traceability-set base_wp_id active_wp_id:
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/wp-traceability-set.mjs {{base_wp_id}} {{active_wp_id}}

# Orchestrator wrapper: create WP worktree + PREPARE record + task packet from the signature bundle.
# Optional workflow lane / execution owner args are accepted for legacy recovery only.
# After creation: commits governance on gov_kernel + creates backup snapshot [CX-212F].
orchestrator-prepare-and-packet wp-id workflow_lane="" execution_lane="" label="pre-wp-launch":
	@just worktree-add {{wp-id}}
	@just record-prepare {{wp-id}} {{workflow_lane}} {{execution_lane}}
	@just create-task-packet {{wp-id}}
	@echo "[ORCHESTRATOR] Committing governance checkpoint on gov_kernel..."
	@cd ../wt-gov-kernel && git add -A && git diff --cached --quiet || git commit -m "gov: checkpoint packet+refinement+micro-tasks [{{wp-id}}]"
	@echo "[ORCHESTRATOR] Creating backup snapshot before coder launch..."
	@just backup-snapshot "{{label}}"

# Orchestrator wrapper: create WP worktree + task packet when PREPARE is already recorded
# (for example, retrying packet creation after a previous blocked attempt).
orchestrator-worktree-and-packet wp-id:
	@just worktree-add {{wp-id}}
	@just create-task-packet {{wp-id}}

# Create new task packet from template [CX-580]
create-task-packet wp-id:
	@echo "Creating task packet: {{wp-id}}..."
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/create-task-packet.mjs {{wp-id}}
	@just build-order-sync

ensure-wp-communications wp-id:
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/ensure-wp-communications.mjs {{wp-id}}

wp-communications-check:
	@node {{GOV_ROOT}}/roles_shared/checks/wp-communications-check.mjs

wp-communication-health-check wp-id stage='STATUS':
	@node {{GOV_ROOT}}/roles_shared/checks/wp-communication-health-check.mjs {{wp-id}} {{stage}}

wp-thread-append wp-id actor-role actor-session message target='' target-role='' target-session='' correlation-id='' requires-ack='false' ack-for='' spec-anchor='' packet-row-ref='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-thread-append.mjs {{wp-id}} {{actor-role}} {{actor-session}} "{{message}}" "{{target}}" "{{target-role}}" "{{target-session}}" "{{correlation-id}}" {{requires-ack}} "{{ack-for}}" "{{spec-anchor}}" "{{packet-row-ref}}"

# Check pending notifications for a role on a WP (or all roles with --all)
check-notifications wp-id role='' *args='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-check-notifications.mjs {{wp-id}} {{role}} {{args}}

# Acknowledge pending notifications for a role on a WP (advances cursor)
ack-notifications wp-id role session='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-check-notifications.mjs {{wp-id}} {{role}} --ack --session={{session}}

wp-receipt-append wp-id actor-role actor-session receipt-kind summary state-before='' state-after='' target-role='' target-session='' correlation-id='' requires-ack='false' ack-for='' spec-anchor='' packet-row-ref='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-receipt-append.mjs {{wp-id}} {{actor-role}} {{actor-session}} {{receipt-kind}} "{{summary}}" "{{state-before}}" "{{state-after}}" "{{target-role}}" "{{target-session}}" "{{correlation-id}}" {{requires-ack}} "{{ack-for}}" "{{spec-anchor}}" "{{packet-row-ref}}"

wp-heartbeat wp-id actor-role actor-session current-phase runtime-status next-expected-actor waiting-on validator-trigger='NONE' last-event='' worktree-dir='' next-expected-session='' waiting-on-session='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-heartbeat.mjs {{wp-id}} {{actor-role}} {{actor-session}} {{current-phase}} {{runtime-status}} {{next-expected-actor}} "{{waiting-on}}" {{validator-trigger}} "{{last-event}}" "{{worktree-dir}}" "{{next-expected-session}}" "{{waiting-on-session}}"

wp-validator-kickoff wp-id actor-session target-session summary correlation-id='' spec-anchor='' packet-row-ref='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs VALIDATOR_KICKOFF {{wp-id}} WP_VALIDATOR {{actor-session}} CODER "{{target-session}}" "{{summary}}" "{{correlation-id}}" "{{spec-anchor}}" "{{packet-row-ref}}"

wp-coder-intent wp-id actor-session target-session summary correlation-id spec-anchor='' packet-row-ref='' ack-for='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs CODER_INTENT {{wp-id}} CODER {{actor-session}} WP_VALIDATOR "{{target-session}}" "{{summary}}" "{{correlation-id}}" "{{spec-anchor}}" "{{packet-row-ref}}" "{{ack-for}}"

wp-coder-handoff wp-id actor-session target-session summary correlation-id='' spec-anchor='' packet-row-ref='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs CODER_HANDOFF {{wp-id}} CODER {{actor-session}} WP_VALIDATOR "{{target-session}}" "{{summary}}" "{{correlation-id}}" "{{spec-anchor}}" "{{packet-row-ref}}"

wp-validator-review wp-id actor-session target-session summary correlation-id spec-anchor='' packet-row-ref='' ack-for='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs VALIDATOR_REVIEW {{wp-id}} WP_VALIDATOR {{actor-session}} CODER "{{target-session}}" "{{summary}}" "{{correlation-id}}" "{{spec-anchor}}" "{{packet-row-ref}}" "{{ack-for}}"

wp-validator-query wp-id actor-role actor-session target-session summary correlation-id='' spec-anchor='' packet-row-ref='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs VALIDATOR_QUERY {{wp-id}} {{actor-role}} {{actor-session}} WP_VALIDATOR "{{target-session}}" "{{summary}}" "{{correlation-id}}" "{{spec-anchor}}" "{{packet-row-ref}}"

wp-validator-response wp-id actor-role actor-session target-session summary correlation-id spec-anchor='' packet-row-ref='' ack-for='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs VALIDATOR_RESPONSE {{wp-id}} {{actor-role}} {{actor-session}} CODER "{{target-session}}" "{{summary}}" "{{correlation-id}}" "{{spec-anchor}}" "{{packet-row-ref}}" "{{ack-for}}"

wp-review-request wp-id actor-role actor-session target-role target-session summary correlation-id='' spec-anchor='' packet-row-ref='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs REVIEW_REQUEST {{wp-id}} {{actor-role}} {{actor-session}} {{target-role}} "{{target-session}}" "{{summary}}" "{{correlation-id}}" "{{spec-anchor}}" "{{packet-row-ref}}"

wp-review-response wp-id actor-role actor-session target-role target-session summary correlation-id spec-anchor='' packet-row-ref='' ack-for='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs REVIEW_RESPONSE {{wp-id}} {{actor-role}} {{actor-session}} {{target-role}} "{{target-session}}" "{{summary}}" "{{correlation-id}}" "{{spec-anchor}}" "{{packet-row-ref}}" "{{ack-for}}"

wp-spec-gap wp-id actor-role actor-session target-role target-session summary correlation-id='' spec-anchor='' packet-row-ref='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs SPEC_GAP {{wp-id}} {{actor-role}} {{actor-session}} {{target-role}} "{{target-session}}" "{{summary}}" "{{correlation-id}}" "{{spec-anchor}}" "{{packet-row-ref}}"

wp-spec-confirmation wp-id actor-role actor-session target-role target-session summary correlation-id spec-anchor='' packet-row-ref='' ack-for='':
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs SPEC_CONFIRMATION {{wp-id}} {{actor-role}} {{actor-session}} {{target-role}} "{{target-session}}" "{{summary}}" "{{correlation-id}}" "{{spec-anchor}}" "{{packet-row-ref}}" "{{ack-for}}"

operator-monitor *args:
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/operator-monitor-tui.mjs {{args}}

operator-admin *args:
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/operator-monitor-tui.mjs --admin {{args}}

# Create new task packet stub from template (backlog; non-executable)
create-task-packet-stub wp-id roadmap_pointer="" line_numbers="":
	@echo "Creating task packet stub: {{wp-id}}..."
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/create-task-packet-stub.mjs {{wp-id}} "{{roadmap_pointer}}" "{{line_numbers}}"
	@just build-order-sync

# Pre-work validation - run before starting implementation [CX-587, CX-620]
pre-work wp-id:
	@node {{GOV_ROOT}}/roles/coder/checks/pre-work.mjs {{wp-id}}

# Post-work validation - run before or after commit [CX-623, CX-651]
post-work wp-id *args:
	@node {{GOV_ROOT}}/roles/coder/checks/post-work.mjs {{wp-id}} {{args}}

# Coder helper: docs-only skeleton checkpoint commit (task packet only).
coder-skeleton-checkpoint wp-id:
	@node {{GOV_ROOT}}/roles/coder/checks/coder-skeleton-checkpoint.mjs {{wp-id}}

# Workflow-authority helper: approve a WP skeleton checkpoint (unblocks implementation).
# In ORCHESTRATOR_MANAGED this may be Orchestrator, Validator, or Operator.
skeleton-approved wp-id:
	@node {{GOV_ROOT}}/roles_shared/checks/skeleton-approved.mjs {{wp-id}}

# Helper: compute deterministic COR-701 Pre/Post SHA1 for a file.
cor701-sha file:
	@node {{GOV_ROOT}}/roles_shared/checks/cor701-sha.mjs {{file}}

# Automated workflow validation for a work packet
validate-workflow wp-id:
	@echo "Running automated workflow validation for {{wp-id}}..."
	@echo ""
	@echo "Step 0: Pre-work check"
	@just pre-work {{wp-id}}
	@echo ""
	@echo "Step 1: Code quality validation"
	@just validate
	@echo ""
	@echo "Step 2: Post-work check"
	@just post-work {{wp-id}}
	@echo ""
	@echo "✅ Automated workflow validation passed for {{wp-id}} (manual review required)"

# Gate check (protocol-aligned)
gate-check wp-id:
	@node {{GOV_ROOT}}/roles_shared/checks/gate-check.mjs {{wp-id}}

# Role Mailbox export gate (RoleMailboxExportGate) [2.6.8.10]
role-mailbox-export-check:
	@node {{GOV_ROOT}}/roles_shared/checks/role_mailbox_export_check.mjs

# Product Governance Snapshot (Spec v02.125 7.5.4.10)
governance-snapshot:
	@node {{GOV_ROOT}}/roles_shared/scripts/governance-snapshot.mjs

validator-governance-snapshot:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-governance-snapshot.mjs

# Validator helpers (protocol-aligned)
validator-scan:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-scan.mjs

# Alias to clarify intent: validator-scan scans product sources.
product-scan:
	@just validator-scan

validator-dal-audit:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-dal-audit.mjs

validator-spec-regression:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-spec-regression.mjs

validator-phase-gate phase="Phase-1":
	@node {{GOV_ROOT}}/roles/validator/checks/validator-phase-gate.mjs {{phase}}

validator-packet-complete wp-id:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-packet-complete.mjs {{wp-id}}

validator-report-structure-check:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-report-structure-check.mjs

validator-error-codes:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-error-codes.mjs

validator-coverage-gaps *targets:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-coverage-gaps.mjs {{targets}}

validator-traceability *targets:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-traceability.mjs {{targets}}

validator-git-hygiene:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-git-hygiene.mjs

validator-hygiene-full:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-hygiene-full.mjs

validator-handoff-check wp-id *args:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-handoff-check.mjs {{wp-id}} {{args}}

external-validator-brief wp-id *args:
	@node {{GOV_ROOT}}/roles/validator/checks/external-validator-brief.mjs {{wp-id}} {{args}}

# Validator Gate Commands [CX-VAL-GATE] - Mechanical enforcement of validation sequence
validator-gate-present wp-id verdict="":
	@node {{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs present-report {{wp-id}} {{verdict}}

validator-gate-acknowledge wp-id:
	@node {{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs acknowledge {{wp-id}}

validator-gate-append wp-id verdict="":
	@node {{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs append {{wp-id}} {{verdict}}

validator-gate-commit wp-id:
	@node {{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs commit {{wp-id}}

validator-gate-status wp-id:
	@node {{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs status {{wp-id}}

validator-gate-reset wp-id *confirm:
	@node {{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs reset {{wp-id}} {{confirm}}
