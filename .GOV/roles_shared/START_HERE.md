# Handshake Project: Start Here

Authority: Master Spec (see `.GOV/roles_shared/SPEC_CURRENT.md`)
---
## Canonical sources
- **Spec:** `.GOV/roles_shared/SPEC_CURRENT.md` (points to the current Handshake master spec).
- **WP Traceability:** `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md` (Base WP â†’ Active Packet mapping; resolves `-vN` revisions without putting WP IDs into the Master Spec).
- **Governance guardrails:** `Handshake Codex v1.4` (repo root) + `.GOV/roles_shared/TASK_BOARD.md` + task packets. Handshake logger is for milestones/hard bugs when requested.
- **Architecture & debug:** `.GOV/roles_shared/ARCHITECTURE.md` and `.GOV/roles_shared/RUNBOOK_DEBUG.md`.

## AI Agent Workflow (Mandatory for AI-Autonomous Operation)

**[CX-503, CX-580-623]** This repository is designed for AI-autonomous software engineering. Human users may not have coding expertise and rely on deterministic workflow enforcement.

**Two agent roles:**
1. **Orchestrator** â€” Creates task packets, delegates work, manages workflow
2. **Coder/Debugger** â€” Implements work per task packet scope

**Mandatory protocols:**
- **Orchestrators:** Read `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` before delegating
- **Coders:** Read `.GOV/roles/coder/CODER_PROTOCOL.md` before writing any code

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

# Full workflow validation (pre-work + validate + post-work)
just validate-workflow WP-{ID}

# Governance-only health check (no product scan)
just gov-check
```

**Governance-only maintenance (no WP required) [CX-111]:**
- Allowed scope (planned diff must be strictly limited to these governance surfaces): `/.GOV/**`, `/.github/**`, `/justfile`, `/Handshake Codex v1.4.md`, `/AGENTS.md`
- Verification: `just gov-check`
- If any product path is touched (`/src/`, `/app/`, `/tests/`): STOP and require a WP + Gate 0/1 (`just pre-work WP-{ID}` / `just post-work WP-{ID}`)

**Gate 0 (Pre-Work):** Task packet MUST exist and pass `just pre-work WP-{ID}` before implementation starts. If blocked, STOP and request help.

**Gate 1 (Post-Work):** All validation MUST pass `just post-work WP-{ID}` before commit. If blocked, fix issues and re-run.

**Gate visibility (chat UX):** when a gate runs (or blocks), paste the verbatim output and immediately follow with a short phase/status + copy/paste next commands (see role protocols).

**See:** `.GOV/roles_shared/QUALITY_GATE.md` for Gate 0 and Gate 1 requirements.

Quick reference:
- `.GOV/roles_shared/ROLE_WORKFLOW_QUICKREF.md` (drive-agnostic role workflow + operator UX)

## Repo map (open in an editor and `rg`)
- `app/` â€” React + Tauri frontend; UI components live under `app/src/`.
- `app/src-tauri/` â€” Tauri shell; spawns `handshake_core` from `src/backend/handshake_core`.
- `src/backend/handshake_core/` â€” Rust backend crate (API, data, logging).
- `src/shared/` â€” placeholder for cross-stack types/contracts (none defined yet).
- `tests/` â€” top-level test harness placeholder.
- `.GOV/scripts/` â€” ops/dev scripts (currently empty scaffold).
- `data/` â€” runtime artifacts; backend logs are written to `data/logs/handshake_core.log`.
- `.GOV/` â€” canonical docs (this pack) + `.GOV/adr/` (accepted ADRs).
- `.GOV/operator/docs_local/` â€” staging/non-canonical notes and diaries.
- `log_archive/` â€” historical logger drops.
- `.GOV/roles_shared/OWNERSHIP.md` â€” path/area owners for routing reviews.
- Root files: `Handshake_Master_Spec_v*.md`, `Handshake Codex v1.4.md`, `Handshake_logger_*`, phase/plan docs.
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` and `.GOV/roles/coder/CODER_PROTOCOL.md` â€” AI agent workflow protocols.

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

# Git hook (pre-commit checks)
git config core.hooksPath .GOV/scripts/hooks
```

### Phase 1 prerequisite: Ollama (local model runtime)
Phase 1 LLM-backed features require Ollama running locally.

Windows setup (manual; run in your own terminal):
1. Install Ollama: `winget install -e --id Ollama.Ollama`
2. Start the server: `ollama serve` (or use `ollama run mistral` for a smoke run + model download)
3. Verify the API: `irm http://localhost:11434/api/tags` (PowerShell) or `curl http://localhost:11434/api/tags`

Handshake environment (optional overrides):
- `OLLAMA_URL` (default: `http://localhost:11434`)
- `OLLAMA_MODEL` (default: `llama3`)

Troubleshooting:
- Port conflict (11434): `netstat -ano | findstr 11434`
- If `just dev` reports an Ollama preflight error, confirm `OLLAMA_URL` and that `ollama serve` is running.

For task packets: include scope, expected behavior, in-scope paths, DONE_MEANS, BOOTSTRAP block (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP), and these commands.

CI expectation: run `just validate`; manual validator review is required for MEDIUM/HIGH risk work.

## Bug triage map (jump into RUNBOOK_DEBUG)
- UI/frontend: see `.GOV/roles_shared/RUNBOOK_DEBUG.md#ui-and-shell` (app React + Tauri window lifecycle).
- Backend/API/logic: see `.GOV/roles_shared/RUNBOOK_DEBUG.md#backend-api-and-logic` (Rust `api/*.rs`, models, logging).
- IPC / orchestrator (Tauri â†” Rust core): see `.GOV/roles_shared/RUNBOOK_DEBUG.md#ipc-tauri-bridge` (`app/src-tauri/src/lib.rs` spawn + commands).
- Data/migrations/storage: see `.GOV/roles_shared/RUNBOOK_DEBUG.md#data-storage-and-migrations` (`migrations/`, SQLite, RDD model).

## More context
- Architecture table: `.GOV/roles_shared/ARCHITECTURE.md`
- Debug runbook: `.GOV/roles_shared/RUNBOOK_DEBUG.md`
- Current spec + governance: `.GOV/roles_shared/SPEC_CURRENT.md`
- Quality gate (risk tiers + required checks): `.GOV/roles_shared/QUALITY_GATE.md`
- Task packet template: `.GOV/templates/TASK_PACKET_TEMPLATE.md`
- Workflow template for reuse: `.GOV/templates/AI_WORKFLOW_TEMPLATE.md`

## Past work
Pointer to prior specs/logs/notes: `.GOV/roles_shared/PAST_WORK_INDEX.md`
