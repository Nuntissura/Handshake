# START_HERE

## Canonical sources
- **Spec:** `docs/SPEC_CURRENT.md` (points to the current Handshake master spec).
- **Governance guardrails:** `Handshake Codex v0.8` (repo root) + latest `Handshake_logger_*` (root + `log_archive/`).
- **Architecture & debug:** `docs/ARCHITECTURE.md` and `docs/RUNBOOK_DEBUG.md`.

## AI Agent Workflow (Mandatory for AI-Autonomous Operation)

**[CX-503, CX-580-623]** This repository is designed for AI-autonomous software engineering. Human users may not have coding expertise and rely on deterministic workflow enforcement.

**Two agent roles:**
1. **Orchestrator** — Creates task packets, delegates work, manages workflow
2. **Coder/Debugger** — Implements work per task packet scope

**Mandatory protocols:**
- **Orchestrators:** Read `docs/ORCHESTRATOR_PROTOCOL.md` before delegating
- **Coders:** Read `docs/CODER_PROTOCOL.md` before writing any code

**Workflow enforcement commands:**
```bash
# Orchestrator: Create task packet from template
just create-task-packet WP-{phase}-{name}

# Orchestrator: Verify packet complete before delegation
just pre-work WP-{ID}

# Coder: Verify packet exists before coding
just pre-work WP-{ID}

# Coder: Verify work complete before commit
just post-work WP-{ID}

# Full workflow validation (pre-work + validate + ai-review + post-work)
just validate-workflow WP-{ID}
```

**Gate 0 (Pre-Work):** Task packet MUST exist and pass `just pre-work WP-{ID}` before implementation starts. If blocked, STOP and request help.

**Gate 1 (Post-Work):** All validation MUST pass `just post-work WP-{ID}` before commit. If blocked, fix issues and re-run.

**See:** `docs/QUALITY_GATE.md` for Gate 0 and Gate 1 requirements.

## Repo map (open in an editor and `rg`)
- `app/` — React + Tauri frontend; UI components live under `app/src/`.
- `app/src-tauri/` — Tauri shell; spawns `handshake_core` from `src/backend/handshake_core`.
- `src/backend/handshake_core/` — Rust backend crate (API, data, logging).
- `src/shared/` — placeholder for cross-stack types/contracts (none defined yet).
- `tests/` — top-level test harness placeholder.
- `scripts/` — ops/dev scripts (currently empty scaffold).
- `data/` — runtime artifacts; backend logs are written to `data/logs/handshake_core.log`.
- `docs/` — canonical docs (this pack) + `docs/adr/` (accepted ADRs).
- `docs_local/` — staging/non-canonical notes and diaries.
- `log_archive/` — historical logger drops.
- `docs/OWNERSHIP.md` — path/area owners for routing reviews.
- Root files: `Handshake_Master_Spec_v*.md`, `Handshake Codex v0.8`, `Handshake_logger_*`, phase/plan docs.
- `docs/ORCHESTRATOR_PROTOCOL.md` and `docs/CODER_PROTOCOL.md` — AI agent workflow protocols.

## How to run
> **WARNING for AI Agents:** Commands like `pnpm -C app tauri dev` or `just dev` start a long-running development server. They MUST NOT be executed with a blocking tool (like `run_shell_command`). These commands should be run in a separate, dedicated terminal by the user or as a true background process.
```bash
# Frontend dev shell (Tauri + React)
pnpm -C app tauri dev

# With just (if installed)
just dev

# Backend tests
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Frontend tests
pnpm -C app test

# Lint
pnpm -C app run lint
# or
just lint

# Full hygiene (lint/tests/depcruise/clippy/deny)
just validate

# Scaffolding
just new-react-component <ComponentName>
just new-api-endpoint <endpoint_name>

# AI review (requires gemini CLI)
just ai-review

# Git hook (auto-run AI review on commit)
git config core.hooksPath scripts/hooks
```
If additional setup (DB seed, env) is required: TBD (HSK-1001) — document once known.

For task packets: include scope, expected behavior, in-scope paths, DONE_MEANS, BOOTSTRAP block (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP), and these commands.

CI expectation: run `just validate`; AI review is local (`just ai-review`) and the output must be recorded in the task packet/logger.

## Bug triage map (jump into RUNBOOK_DEBUG)
- UI/frontend: see `docs/RUNBOOK_DEBUG.md#ui-and-shell` (app React + Tauri window lifecycle).
- Backend/API/logic: see `docs/RUNBOOK_DEBUG.md#backend-api-and-logic` (Rust `api/*.rs`, models, logging).
- IPC / orchestrator (Tauri ↔ Rust core): see `docs/RUNBOOK_DEBUG.md#ipc-tauri-bridge` (`app/src-tauri/src/lib.rs` spawn + commands).
- Data/migrations/storage: see `docs/RUNBOOK_DEBUG.md#data-storage-and-migrations` (`migrations/`, SQLite, RDD model).

## More context
- Architecture table: `docs/ARCHITECTURE.md`
- Debug runbook: `docs/RUNBOOK_DEBUG.md`
- Current spec + governance: `docs/SPEC_CURRENT.md`
- Quality gate (risk tiers + required checks): `docs/QUALITY_GATE.md`
- Task packet template: `docs/TASK_PACKET_TEMPLATE.md`
- Workflow template for reuse: `docs/AI_WORKFLOW_TEMPLATE.md`

## Past work
Pointer to prior specs/logs/notes: `docs/PAST_WORK_INDEX.md`
