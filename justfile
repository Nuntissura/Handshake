set dotenv-load := false
# Use a Windows-friendly shell if available; defaults remain for *nix.
# Powershell is present on Windows by default.
set windows-shell := ["powershell.exe", "-NoLogo", "-NonInteractive", "-Command"]

dev: preflight-ollama
	cd app; pnpm run tauri dev

# Fail fast if Ollama is missing/unreachable (Phase 1 requirement; see .GOV/roles_shared/START_HERE.md).
preflight-ollama:
	node -e "const base=(process.env.OLLAMA_URL||'http://localhost:11434'); const normalized=base.endsWith('/')?base.slice(0,-1):base; const url=normalized + '/api/tags'; const lib=url.startsWith('https://')?require('https'):require('http'); const req=lib.get(url,(res)=>{ const ok=!!res.statusCode && res.statusCode>=200 && res.statusCode<300; if(ok){ process.exit(0); } console.error('Ollama preflight failed: GET ' + url + ' returned ' + res.statusCode + '. Install Ollama (Windows: winget install -e --id Ollama.Ollama), then run ollama serve (or ollama run mistral), or set OLLAMA_URL.'); process.exit(1); }); req.on('error',()=>{ console.error('Ollama preflight failed: cannot reach ' + url + '. Install Ollama (Windows: winget install -e --id Ollama.Ollama), then run ollama serve (or ollama run mistral), or set OLLAMA_URL.'); process.exit(1); }); req.setTimeout(3000, ()=>req.destroy(new Error('timeout')));"

lint:
	cd app; pnpm run lint
	cd src/backend/handshake_core; cargo clippy --all-targets --all-features

test:
	cd src/backend/handshake_core; cargo test

# Fail if any required docs are missing (navigation pack + past work index)
docs-check:
	node -e "['.GOV/roles_shared/START_HERE.md', '.GOV/roles_shared/SPEC_CURRENT.md', '.GOV/roles_shared/ARCHITECTURE.md', '.GOV/roles_shared/RUNBOOK_DEBUG.md', '.GOV/roles_shared/PAST_WORK_INDEX.md'].forEach(f => { if (!require('fs').existsSync(f)) { console.error('Missing: ' + f); process.exit(1); } })"

# Format backend Rust
fmt:
	cd src/backend/handshake_core; cargo fmt

# Clean Cargo artifacts in the external target dir (../Cargo Target/handshake-cargo-target)
cargo-clean:
	cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Cargo Target/handshake-cargo-target"

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
	cd src/backend/handshake_core; cargo clippy --all-targets --all-features
	cd src/backend/handshake_core; cargo test
	cargo deny --manifest-path src/backend/handshake_core/Cargo.toml check advisories licenses bans sources

# Codex guardrails: prevent direct fetch in components, println/eprintln in backend, and doc drift.
codex-check:
	node .GOV/scripts/validation/codex-check.mjs

# Worktrees (recommended when >1 WP active)
# Creates a dedicated working directory for the WP branch.
worktree-add wp-id base="main" branch="" dir="":
	node .GOV/scripts/worktree-add.mjs {{wp-id}} {{base}} {{branch}} {{dir}}

# Hard gate helper: Worktree + Branch Gate [CX-WT-001]
hard-gate-wt-001:
	@echo 'ROLE=<ROLE> | PROTOCOL=<protocol path> | COMPLIANCE={OK|BLOCKED} | NEXT_ACTION=<short>'
	@echo ''
	@echo 'LIFECYCLE [CX-LIFE-001]'
	@echo '- WP_ID: <WP-... or N/A>'
	@echo '- STAGE: <REFINEMENT|SIGNATURE|PREPARE|PACKET_CREATE|DELEGATION|STATUS_SYNC>'
	@echo '- NEXT: <next stage or STOP>'
	@echo ''
	@echo 'STATE: <1 sentence>'
	@echo ''
	@echo 'HARD_GATE_OUTPUT [CX-WT-001]'
	@pwd
	@git rev-parse --show-toplevel
	@git rev-parse --abbrev-ref HEAD
	@git status -sb
	@git worktree list
	@echo ''
	@echo 'HARD_GATE_REASON [CX-WT-001]'
	@echo '- Prevent edits in the wrong repo/worktree directory.'
	@echo '- Prevent accidental work on the wrong branch (e.g., `main`/role branches).'
	@echo '- Enforce WP isolation: one WP == one worktree + branch.'
	@echo '- Avoid cross-WP contamination of unstaged changes and commits.'
	@echo '- Ensure deterministic handoff: Operator/Validator can verify state without back-and-forth.'
	@echo '- Provide a verifiable snapshot for audits and validation evidence.'
	@echo '- Catch missing/mispointed worktrees early (before any changes).'
	@echo '- Ensure `git worktree list` topology matches concurrency expectations.'
	@echo '- Prevent using the Operator''s personal worktree as a Coder worktree.'
	@echo '- Ensure the Orchestrator''s assignment is actually in effect locally.'
	@echo '- Bind Coder work to `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` `PREPARE` records (`branch`, `worktree_dir`).'
	@echo '- Keep role-governed defaults consistent with `.GOV/roles_shared/ROLE_WORKTREES.md`.'
	@echo '- Reduce risk of data loss from wrong-directory "cleanup"/stashing mistakes.'
	@echo '- Make failures actionable: mismatch => STOP + escalate, not "guess and proceed".'
	@echo ''
	@echo 'HARD_GATE_NEXT_ACTIONS [CX-WT-001]'
	@echo '- If correct (repo/worktree/branch match the assignment): proceed to BOOTSTRAP / packet steps.'
	@echo '- If incorrect/uncertain: STOP; ask Orchestrator/Operator to provide/create the correct WP worktree/branch and ensure `PREPARE` is recorded in `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`.'
	@echo ''
	@echo 'PHASE_STATUS [CX-GATE-UX-001]'
	@echo '- PHASE: <REFINEMENT|SIGNATURE|PREPARE|PACKET_CREATE|DELEGATION|STATUS_SYNC>'
	@echo '- DECISION: <PROCEED|STOP>'
	@echo '- NEXT_COMMANDS:'
	@echo '  - <cmd1>'
	@echo '  - <cmd2>'

task-board-check:
	node .GOV/scripts/validation/task-board-check.mjs

task-packet-claim-check:
	node .GOV/scripts/validation/task-packet-claim-check.mjs

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
close-wp-branch wp-id remote="":
	node .GOV/scripts/close-wp-branch.mjs {{wp-id}} {{remote}}

# === Workflow Enforcement Commands (see .GOV/roles_shared/SPEC_CURRENT.md) ===

# Record a technical refinement for a work packet [CX-585A]
record-refinement wp-id detail="":
	@node .GOV/scripts/validation/orchestrator_gates.mjs refine {{wp-id}} "{{detail}}"

# Record a user signature for a work packet [CX-585C]
record-signature wp-id signature:
	@node .GOV/scripts/validation/orchestrator_gates.mjs sign {{wp-id}} {{signature}}

# Record WP preparation (branch/worktree + coder assignment) after signature and before packet creation.
record-prepare wp-id coder_id branch="" worktree_dir="":
	@node .GOV/scripts/validation/orchestrator_gates.mjs prepare {{wp-id}} {{coder_id}} {{branch}} {{worktree_dir}}

# Create new task packet from template [CX-580]
create-task-packet wp-id:
	@echo "Creating task packet: {{wp-id}}..."
	@node .GOV/scripts/create-task-packet.mjs {{wp-id}}

# Pre-work validation - run before starting implementation [CX-587, CX-620]
pre-work wp-id:
	@just gate-check {{wp-id}}
	@node .GOV/scripts/validation/pre-work-check.mjs {{wp-id}}

# Post-work validation - run before or after commit [CX-623, CX-651]
post-work wp-id *args:
	@just gate-check {{wp-id}}
	@node .GOV/scripts/validation/post-work-check.mjs {{wp-id}} {{args}}
	@just role-mailbox-export-check

# Helper: compute deterministic COR-701 Pre/Post SHA1 for a file.
cor701-sha file:
	@node .GOV/scripts/validation/cor701-sha.mjs {{file}}

# Automated workflow validation for a work packet
validate-workflow wp-id:
	@echo "Running automated workflow validation for {{wp-id}}..."
	@echo ""
	@echo "Step 0: Gate Check"
	@just gate-check {{wp-id}}
	@echo ""
	@echo "Step 1: Pre-work check"
	@just pre-work {{wp-id}}
	@echo ""
	@echo "Step 2: Code quality validation"
	@just validate
	@echo ""
	@echo "Step 3: Post-work check"
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
validator-gate-present wp-id verdict:
	@node .GOV/scripts/validation/validator_gates.mjs present-report {{wp-id}} {{verdict}}

validator-gate-acknowledge wp-id:
	@node .GOV/scripts/validation/validator_gates.mjs acknowledge {{wp-id}}

validator-gate-append wp-id:
	@node .GOV/scripts/validation/validator_gates.mjs append {{wp-id}}

validator-gate-commit wp-id:
	@node .GOV/scripts/validation/validator_gates.mjs commit {{wp-id}}

validator-gate-status wp-id:
	@node .GOV/scripts/validation/validator_gates.mjs status {{wp-id}}

validator-gate-reset wp-id *confirm:
	@node .GOV/scripts/validation/validator_gates.mjs reset {{wp-id}} {{confirm}}
