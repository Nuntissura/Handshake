set dotenv-load := false
# Use a Windows-friendly shell if available; defaults remain for *nix.
# Powershell is present on Windows by default.
set windows-shell := ["powershell.exe", "-NoLogo", "-NonInteractive", "-Command"]

dev:
	cd app; pnpm run tauri dev

lint:
	cd app; pnpm run lint
	cd src/backend/handshake_core; cargo clippy --all-targets --all-features

test:
	cd src/backend/handshake_core; cargo test

# Fail if any required docs are missing (navigation pack + past work index)
docs-check:
	node -e "['docs/START_HERE.md', 'docs/SPEC_CURRENT.md', 'docs/ARCHITECTURE.md', 'docs/RUNBOOK_DEBUG.md', 'docs/PAST_WORK_INDEX.md'].forEach(f => { if (!require('fs').existsSync(f)) { console.error('Missing: ' + f); process.exit(1); } })"

# Format backend Rust
fmt:
	cd src/backend/handshake_core; cargo fmt

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
	cargo deny check advisories licenses bans sources

# Codex guardrails: prevent direct fetch in components, println/eprintln in backend, and doc drift.
codex-check:
	node scripts/validation/codex-check.mjs

# Dependency cruise (frontend architecture)
depcruise:
	cd app; pnpm run depcruise

# Dependency & license checks (Rust)
deny:
	cargo deny check advisories licenses bans sources

# Scaffolding
new-react-component name:
	node scripts/new-react-component.mjs {{name}}

new-api-endpoint name:
	node scripts/new-api-endpoint.mjs {{name}}

scaffold-check:
	node scripts/scaffold-check.mjs

codex-check-test:
	node scripts/codex-check-test.mjs

# AI review (requires gemini CLI)
ai-review:
	node scripts/ai-review-gemini.mjs

# === Workflow Enforcement Commands (Codex v0.8) ===

# Create new task packet from template [CX-580]
create-task-packet wp-id:
	@echo "Creating task packet: {{wp-id}}..."
	@node scripts/create-task-packet.mjs {{wp-id}}

# Pre-work validation - run before starting implementation [CX-587, CX-620]
pre-work wp-id:
	@just gate-check {{wp-id}}
	@node scripts/validation/pre-work-check.mjs {{wp-id}}

# Post-work validation - run before commit [CX-623, CX-651]
post-work wp-id:
	@just gate-check {{wp-id}}
	@node scripts/validation/post-work-check.mjs {{wp-id}}

# Full workflow validation for a work packet
validate-workflow wp-id:
	@echo "Running full workflow validation for {{wp-id}}..."
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
	@echo "Step 3: AI review"
	@just ai-review
	@echo ""
	@echo "Step 4: Post-work check"
	@just post-work {{wp-id}}
	@echo ""
	@echo "✅ Full workflow validation passed for {{wp-id}}"

# Gate check (protocol-aligned)
gate-check wp-id:
	@node scripts/validation/gate-check.mjs {{wp-id}}

# Validator helpers (protocol-aligned)
validator-scan:
	@node scripts/validation/validator-scan.mjs

validator-dal-audit:
	@node scripts/validation/validator-dal-audit.mjs

validator-spec-regression:
	@node scripts/validation/validator-spec-regression.mjs

validator-phase-gate phase="Phase-1":
	@node scripts/validation/validator-phase-gate.mjs {{phase}}

validator-packet-complete wp-id:
	@node scripts/validation/validator-packet-complete.mjs {{wp-id}}

validator-error-codes:
	@node scripts/validation/validator-error-codes.mjs

validator-coverage-gaps *targets:
	@node scripts/validation/validator-coverage-gaps.mjs {{targets}}

validator-traceability *targets:
	@node scripts/validation/validator-traceability.mjs {{targets}}

validator-git-hygiene:
	@node scripts/validation/validator-git-hygiene.mjs

validator-hygiene-full:
	@node scripts/validation/validator-hygiene-full.mjs