set dotenv-load := false
# Use a Windows-friendly shell if available; defaults remain for *nix.
# Powershell is present on Windows by default.
set windows-shell := ["powershell.exe", "-NoLogo", "-NonInteractive", "-Command"]

# External build/test artifacts (Cargo target dir) MUST live outside the repo working tree.
CARGO_TARGET_DIR := "../Handshake Artifacts/handshake-cargo-target"

dev: preflight-ollama
	node -e "const {execFileSync}=require('child_process'); const path=require('path'); const repo=execFileSync('git',['rev-parse','--show-toplevel'],{encoding:'utf8'}).trim(); const cargoTarget=path.resolve(repo,'{{CARGO_TARGET_DIR}}'); execFileSync('pnpm',['-C','app','run','tauri','dev'],{stdio:'inherit', env:{...process.env, CARGO_TARGET_DIR:cargoTarget}});"

# Fail fast if Ollama is missing/unreachable (Phase 1 requirement; see .GOV/roles_shared/START_HERE.md).
preflight-ollama:
	node -e "const base=(process.env.OLLAMA_URL||'http://localhost:11434'); const normalized=base.endsWith('/')?base.slice(0,-1):base; const url=normalized + '/api/tags'; const lib=url.startsWith('https://')?require('https'):require('http'); const req=lib.get(url,(res)=>{ const ok=!!res.statusCode && res.statusCode>=200 && res.statusCode<300; if(ok){ process.exit(0); } console.error('Ollama preflight failed: GET ' + url + ' returned ' + res.statusCode + '. Install Ollama (Windows: winget install -e --id Ollama.Ollama), then run ollama serve (or ollama run mistral), or set OLLAMA_URL.'); process.exit(1); }); req.on('error',()=>{ console.error('Ollama preflight failed: cannot reach ' + url + '. Install Ollama (Windows: winget install -e --id Ollama.Ollama), then run ollama serve (or ollama run mistral), or set OLLAMA_URL.'); process.exit(1); }); req.setTimeout(3000, ()=>req.destroy(new Error('timeout')));"

lint:
	cd app; pnpm run lint
	cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features --target-dir "{{CARGO_TARGET_DIR}}"

test:
	cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "{{CARGO_TARGET_DIR}}"

# Fail if any required docs are missing (navigation pack + past work index)
docs-check:
	node -e "['.GOV/roles_shared/START_HERE.md', '.GOV/roles_shared/SPEC_CURRENT.md', '.GOV/roles_shared/ARCHITECTURE.md', '.GOV/roles_shared/RUNBOOK_DEBUG.md', '.GOV/roles_shared/PAST_WORK_INDEX.md', '.GOV/roles_shared/REPO_RESILIENCE.md'].forEach(f => { if (!require('fs').existsSync(f)) { console.error('Missing: ' + f); process.exit(1); } })"

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
	node .GOV/scripts/validation/codex-check.mjs

# Governance-only checks (drive-agnostic + lifecycle UX + task board integrity).
gov-check:
	just docs-check
	node .GOV/scripts/validation/gov-check.mjs

topology-registry-sync:
	node .GOV/scripts/topology-registry-sync.mjs

topology-registry-check:
	node .GOV/scripts/validation/topology-registry-check.mjs

# Safety backup push: push the current committed branch state to its matching GitHub backup branch.
backup-push local_branch="" remote_branch="":
	node .GOV/scripts/backup-push.mjs {{local_branch}} {{remote_branch}}

# Ensure the permanent GitHub backup branches exist, seeded from local main when missing.
ensure-permanent-backup-branches:
	node .GOV/scripts/ensure-permanent-backup-branches.mjs

# Immutable out-of-repo snapshot: git bundles + copied working files.
backup-snapshot label="manual" out_root="" nas_root="":
	node .GOV/scripts/backup-snapshot.mjs --label "{{label}}" --out-root "{{out_root}}" --nas-root "{{nas_root}}"

# Read-only status for local/NAS backup roots and latest snapshot presence.
backup-status:
	node .GOV/scripts/backup-status.mjs

# Immutable snapshot using the configured HANDSHAKE_NAS_BACKUP_ROOT.
backup-snapshot-nas label="manual":
	node .GOV/scripts/backup-snapshot.mjs --label "{{label}}" --require-nas

# Fast-forward the permanent local clones from their matching remotes when all are clean.
sync-all-role-worktrees:
	node .GOV/scripts/sync-all-role-worktrees.mjs

# Enumerate deletable local worktrees/branches and non-protected remote branches with exact approval examples.
enumerate-cleanup-targets:
	node .GOV/scripts/enumerate-cleanup-targets.mjs

# Delete a non-protected git-managed local worktree safely: immutable snapshot first, then git worktree remove only.
delete-local-worktree worktree_id approval:
	node .GOV/scripts/delete-local-worktree.mjs {{worktree_id}} --approve "{{approval}}"

# Master Spec EOF appendix blocks check (Spec §12).
spec-eof-appendices-check:
	node .GOV/scripts/validation/spec-eof-appendices-check.mjs

# Governance sync helper: refresh derived governance views then validate.
gov-sync:
	just build-order-sync
	just topology-registry-sync
	just gov-check

# Build order (derived view) maintenance [CX-BO-001]
build-order-sync:
	node .GOV/scripts/build-order-sync.mjs

build-order-check:
	node .GOV/scripts/validation/build-order-check.mjs

# Worktrees (recommended when >1 WP active)
# Creates a dedicated working directory for the WP branch.
worktree-add wp-id base="main" branch="" dir="":
	node .GOV/scripts/worktree-add.mjs {{wp-id}} {{base}} {{branch}} {{dir}}

# Role-scoped WP worktrees for ORCHESTRATOR_MANAGED CLI sessions.
coder-worktree-add wp-id branch="" dir="":
	node .GOV/scripts/role-session-worktree-add.mjs CODER {{wp-id}} {{branch}} {{dir}}

wp-validator-worktree-add wp-id branch="" dir="":
	node .GOV/scripts/role-session-worktree-add.mjs WP_VALIDATOR {{wp-id}} {{branch}} {{dir}}

integration-validator-worktree-add wp-id branch="" dir="":
	node .GOV/scripts/role-session-worktree-add.mjs INTEGRATION_VALIDATOR {{wp-id}} {{branch}} {{dir}}

# CLI session launch helpers. AUTO prefers repo policy, then falls back to Windows Terminal or print-only.
launch-coder-session wp-id host="AUTO" model="PRIMARY":
	node .GOV/scripts/launch-cli-session.mjs CODER {{wp-id}} {{host}} {{model}}

launch-wp-validator-session wp-id host="AUTO" model="PRIMARY":
	node .GOV/scripts/launch-cli-session.mjs WP_VALIDATOR {{wp-id}} {{host}} {{model}}

launch-integration-validator-session wp-id host="AUTO" model="PRIMARY":
	node .GOV/scripts/launch-cli-session.mjs INTEGRATION_VALIDATOR {{wp-id}} {{host}} {{model}}

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
# Note: using 3 fixed args avoids shell splitting on space-containing filenames.
protocol-ack codex agents protocol:
	@node .GOV/scripts/protocol-ack.mjs "{{codex}}" "{{agents}}" "{{protocol}}"

task-board-check:
	node .GOV/scripts/validation/task-board-check.mjs

task-packet-claim-check:
	node .GOV/scripts/validation/task-packet-claim-check.mjs

phase1-add-coverage-check:
	node .GOV/scripts/validation/phase1-add-coverage-check.mjs

# Dependency cruise (frontend architecture)
depcruise:
	cd app; pnpm run depcruise

# Dependency & license checks (Rust)
deny:
	cargo deny --manifest-path src/backend/handshake_core/Cargo.toml check advisories licenses bans sources

# Scaffolding
new-react-component name:
	node .GOV/scripts/new-react-component.mjs {{name}}

new-api-endpoint name:
	node .GOV/scripts/new-api-endpoint.mjs {{name}}

scaffold-check:
	node .GOV/scripts/scaffold-check.mjs

codex-check-test:
	node .GOV/scripts/codex-check-test.mjs

# Close a WP branch after it has been merged into main.
close-wp-branch wp-id remote="" approval="":
	node .GOV/scripts/close-wp-branch.mjs {{wp-id}} {{remote}} --approve "{{approval}}"

# === Workflow Enforcement Commands (see .GOV/roles_shared/SPEC_CURRENT.md) ===

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
	@just protocol-ack "Handshake Codex v1.4.md" "AGENTS.md" ".GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md"
	@just backup-status
	@just orchestrator-preflight
	@echo 'RESUME_HINT: After a reset/compaction, run `just orchestrator-next [WP-{ID}]` and continue automatically when OPERATOR_ACTION: NONE.'

validator-startup:
	@just protocol-ack "Handshake Codex v1.4.md" "AGENTS.md" ".GOV/roles/validator/VALIDATOR_PROTOCOL.md"
	@just backup-status
	@just validator-preflight
	@echo 'RESUME_HINT: After a reset/compaction, run `just validator-next [WP-{ID}]` and continue automatically when OPERATOR_ACTION: NONE.'

coder-startup:
	@just protocol-ack "Handshake Codex v1.4.md" "AGENTS.md" ".GOV/roles/coder/CODER_PROTOCOL.md"
	@just backup-status
	@just coder-preflight
	@echo 'RESUME_HINT: After a reset/compaction, run `just coder-next [WP-{ID}]` and continue automatically when OPERATOR_ACTION: NONE.'

# Record a technical refinement for a work packet [CX-585A]
record-refinement wp-id detail="":
	@node .GOV/scripts/validation/orchestrator_gates.mjs refine {{wp-id}} "{{detail}}"

# Record a user signature bundle for a work packet [CX-585C]
# Current workflow requires: workflow lane + execution owner.
# Legacy recovery still accepts the older single execution-lane form.
# Allowed workflow lanes: MANUAL_RELAY | ORCHESTRATOR_MANAGED
# Allowed execution owners for current runs: Coder-A .. Coder-Z
record-signature wp-id signature workflow_lane="" execution_lane="":
	@node .GOV/scripts/validation/orchestrator_gates.mjs sign {{wp-id}} {{signature}} {{workflow_lane}} {{execution_lane}}

# Record WP preparation (branch/worktree + execution owner) after signature and before packet creation.
# If omitted, workflow lane / execution owner are inferred from the signed bundle.
record-prepare wp-id workflow_lane="" execution_lane="" branch="" worktree_dir="":
	@node .GOV/scripts/validation/orchestrator_gates.mjs prepare {{wp-id}} {{workflow_lane}} {{execution_lane}} {{branch}} {{worktree_dir}}

# Orchestrator helper (read-only): infer next steps for a WP from gates + file state.
orchestrator-next wp-id="":
	@node .GOV/scripts/orchestrator-next.mjs {{wp-id}}

# Coder helper (read-only): infer next steps for the current WP after reset/compaction.
coder-next wp-id="":
	@node .GOV/scripts/coder-next.mjs {{wp-id}}

# Validator helper (read-only): infer next steps for the current WP after reset/compaction.
validator-next wp-id="":
	@node .GOV/scripts/validator-next.mjs {{wp-id}}

# Deterministic Task Board updater: move a WP entry between sections.
task-board-set wp-id status reason="":
	@node .GOV/scripts/task-board-set.mjs {{wp-id}} {{status}} "{{reason}}"

# Deterministic traceability mapping updater: set Base WP -> Active Packet.
wp-traceability-set base_wp_id active_wp_id:
	@node .GOV/scripts/wp-traceability-set.mjs {{base_wp_id}} {{active_wp_id}}

# Orchestrator wrapper: create WP worktree + PREPARE record + task packet from the signature bundle.
# Optional workflow lane / execution owner args are accepted for legacy recovery only.
orchestrator-prepare-and-packet wp-id workflow_lane="" execution_lane="":
	@just worktree-add {{wp-id}}
	@just record-prepare {{wp-id}} {{workflow_lane}} {{execution_lane}}
	@just create-task-packet {{wp-id}}

# Orchestrator wrapper: create WP worktree + task packet when PREPARE is already recorded
# (for example, retrying packet creation after a previous blocked attempt).
orchestrator-worktree-and-packet wp-id:
	@just worktree-add {{wp-id}}
	@just create-task-packet {{wp-id}}

# Create new task packet from template [CX-580]
create-task-packet wp-id:
	@echo "Creating task packet: {{wp-id}}..."
	@node .GOV/scripts/create-task-packet.mjs {{wp-id}}
	@just build-order-sync

ensure-wp-communications wp-id:
	@node .GOV/scripts/ensure-wp-communications.mjs {{wp-id}}

wp-communications-check:
	@node .GOV/scripts/validation/wp-communications-check.mjs

wp-thread-append wp-id actor-role actor-session message target='':
	@node .GOV/scripts/wp-thread-append.mjs {{wp-id}} {{actor-role}} {{actor-session}} "{{message}}" "{{target}}"

wp-receipt-append wp-id actor-role actor-session receipt-kind summary state-before='' state-after='':
	@node .GOV/scripts/wp-receipt-append.mjs {{wp-id}} {{actor-role}} {{actor-session}} {{receipt-kind}} "{{summary}}" "{{state-before}}" "{{state-after}}"

wp-heartbeat wp-id actor-role actor-session current-phase runtime-status next-expected-actor waiting-on validator-trigger='NONE' last-event='' worktree-dir='':
	@node .GOV/scripts/wp-heartbeat.mjs {{wp-id}} {{actor-role}} {{actor-session}} {{current-phase}} {{runtime-status}} {{next-expected-actor}} "{{waiting-on}}" {{validator-trigger}} "{{last-event}}" "{{worktree-dir}}"

operator-monitor *args:
	@node .GOV/scripts/operator-monitor-tui.mjs {{args}}

# Create new task packet stub from template (backlog; non-executable)
create-task-packet-stub wp-id roadmap_pointer="" line_numbers="":
	@echo "Creating task packet stub: {{wp-id}}..."
	@node .GOV/scripts/create-task-packet-stub.mjs {{wp-id}} "{{roadmap_pointer}}" "{{line_numbers}}"
	@just build-order-sync

# Pre-work validation - run before starting implementation [CX-587, CX-620]
pre-work wp-id:
	@node .GOV/scripts/validation/pre-work.mjs {{wp-id}}

# Post-work validation - run before or after commit [CX-623, CX-651]
post-work wp-id *args:
	@node .GOV/scripts/validation/post-work.mjs {{wp-id}} {{args}}

# Coder helper: docs-only skeleton checkpoint commit (task packet only).
coder-skeleton-checkpoint wp-id:
	@node .GOV/scripts/validation/coder-skeleton-checkpoint.mjs {{wp-id}}

# Workflow-authority helper: approve a WP skeleton checkpoint (unblocks implementation).
# In ORCHESTRATOR_MANAGED this may be Orchestrator, Validator, or Operator.
skeleton-approved wp-id:
	@node .GOV/scripts/validation/skeleton-approved.mjs {{wp-id}}

# Helper: compute deterministic COR-701 Pre/Post SHA1 for a file.
cor701-sha file:
	@node .GOV/scripts/validation/cor701-sha.mjs {{file}}

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
	@node .GOV/scripts/validation/gate-check.mjs {{wp-id}}

# Role Mailbox export gate (RoleMailboxExportGate) [2.6.8.10]
role-mailbox-export-check:
	@node .GOV/scripts/validation/role_mailbox_export_check.mjs

# Product Governance Snapshot (Spec v02.125 7.5.4.10)
governance-snapshot:
	@node .GOV/scripts/governance-snapshot.mjs

validator-governance-snapshot:
	@node .GOV/scripts/validation/validator-governance-snapshot.mjs

# Validator helpers (protocol-aligned)
validator-scan:
	@node .GOV/scripts/validation/validator-scan.mjs

# Alias to clarify intent: validator-scan scans product sources.
product-scan:
	@just validator-scan

validator-dal-audit:
	@node .GOV/scripts/validation/validator-dal-audit.mjs

validator-spec-regression:
	@node .GOV/scripts/validation/validator-spec-regression.mjs

validator-phase-gate phase="Phase-1":
	@node .GOV/scripts/validation/validator-phase-gate.mjs {{phase}}

validator-packet-complete wp-id:
	@node .GOV/scripts/validation/validator-packet-complete.mjs {{wp-id}}

validator-error-codes:
	@node .GOV/scripts/validation/validator-error-codes.mjs

validator-coverage-gaps *targets:
	@node .GOV/scripts/validation/validator-coverage-gaps.mjs {{targets}}

validator-traceability *targets:
	@node .GOV/scripts/validation/validator-traceability.mjs {{targets}}

validator-git-hygiene:
	@node .GOV/scripts/validation/validator-git-hygiene.mjs

validator-hygiene-full:
	@node .GOV/scripts/validation/validator-hygiene-full.mjs

# Validator Gate Commands [CX-VAL-GATE] - Mechanical enforcement of validation sequence
validator-gate-present wp-id verdict="":
	@node .GOV/scripts/validation/validator_gates.mjs present-report {{wp-id}} {{verdict}}

validator-gate-acknowledge wp-id:
	@node .GOV/scripts/validation/validator_gates.mjs acknowledge {{wp-id}}

validator-gate-append wp-id verdict="":
	@node .GOV/scripts/validation/validator_gates.mjs append {{wp-id}} {{verdict}}

validator-gate-commit wp-id:
	@node .GOV/scripts/validation/validator_gates.mjs commit {{wp-id}}

validator-gate-status wp-id:
	@node .GOV/scripts/validation/validator_gates.mjs status {{wp-id}}

validator-gate-reset wp-id *confirm:
	@node .GOV/scripts/validation/validator_gates.mjs reset {{wp-id}} {{confirm}}
