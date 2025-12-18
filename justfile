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
	@rg -n "\\bfetch\\s*\\(" app/src | rg -v "app/src/lib/api.ts" && exit 1 || exit 0
	@echo "Codex check: disallow println!/eprintln! in backend..."
	@rg -n "eprintln!|println!" src/backend/handshake_core/src && exit 1 || exit 0
	@echo "Codex check: docs must reference Codex v0.7..."
	@rg -n "Codex v0\\.5|Handshake Codex v0\\.5" docs && exit 1 || exit 0
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
