set dotenv-load := false

dev:
	cd app && pnpm run tauri dev

lint:
	cd app && pnpm run lint
	cd src/backend/handshake_core && cargo clippy --all-targets --all-features

test:
	cd src/backend/handshake_core && cargo test

# Fail if any required docs are missing (navigation pack + past work index)
docs-check:
	test -s docs/START_HERE.md
	test -s docs/SPEC_CURRENT.md
	test -s docs/ARCHITECTURE.md
	test -s docs/RUNBOOK_DEBUG.md
	test -s docs/PAST_WORK_INDEX.md

# Format backend Rust
fmt:
	cd src/backend/handshake_core && cargo fmt

# Full hygiene pass: docs, lint, tests, fmt, clippy
validate:
	just docs-check
	just codex-check
	just scaffold-check
	just codex-check-test
	cd app && pnpm run lint
	cd app && pnpm test
	cd app && pnpm run depcruise
	cd src/backend/handshake_core && cargo fmt
	cd src/backend/handshake_core && cargo clippy --all-targets --all-features
	cd src/backend/handshake_core && cargo test
	cargo deny check advisories licenses bans sources

# Codex guardrails: prevent direct fetch in components, println/eprintln in backend, and doc drift.
codex-check:
	@echo "Codex check: disallow direct fetch in app/src (outside lib/api.ts)..."
	@rg -n "\\bfetch\\s*\\(" app/src --glob "!app/src/lib/api.ts" && exit 1 || exit 0
	@echo "Codex check: disallow println!/eprintln! in backend..."
	@rg -n "eprintln!|println!" src/backend/handshake_core/src && exit 1 || exit 0
	@echo "Codex check: docs must reference Codex v0.8..."
	@rg -q 'Codex v0\\.8|Handshake Codex v0\\.8' docs && exit 0 || exit 1
	@echo "Codex check: SPEC_CURRENT points to latest master spec..."
	@node scripts/spec-current-check.mjs
	@echo "Codex check: TODOs must include HSK issue tags..."
	@rg -n --pcre2 "TODO(?!\\(HSK-\\d+\\))" app/src src/backend scripts --glob "!scripts/fixtures/**" --glob "!scripts/codex-check-test.mjs" && exit 1 || exit 0

# Dependency cruise (frontend architecture)
depcruise:
	cd app && pnpm run depcruise

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
	@node scripts/validation/pre-work-check.mjs {{wp-id}}

# Post-work validation - run before commit [CX-623, CX-651]
post-work wp-id:
	@node scripts/validation/post-work-check.mjs {{wp-id}}

# Full workflow validation for a work packet
validate-workflow wp-id:
	@echo "Running full workflow validation for {{wp-id}}..."
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
