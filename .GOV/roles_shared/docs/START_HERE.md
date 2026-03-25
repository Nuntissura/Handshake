# Handshake Project: Start Here

Navigation entrypoint only.
Product authority: Master Spec (see `.GOV/spec/SPEC_CURRENT.md`)
Governance placement law: `.GOV/codex/Handshake_Codex_v1.4.md` plus the active role protocols
---
## Canonical sources
- **Spec:** `.GOV/spec/SPEC_CURRENT.md` (points to the current Handshake master spec).
- **Folder-placement law:** `.GOV/codex/Handshake_Codex_v1.4.md` + `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` + `.GOV/roles/coder/CODER_PROTOCOL.md` + `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`.
- **Spec EOF appendices:** Master Spec Â§12 (Feature Registry, Primitive/Tool/Tech Matrix, UI Guidance, Interaction Matrix). These blocks are spec-internal and kept at end-of-file; `just gov-check` enforces presence + parseability.
- **WP Traceability:** `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` (Base WP â†’ Active Packet mapping; resolves `-vN` revisions without putting WP IDs into the Master Spec).
- **Governance guardrails:** `Handshake Codex v1.4` (repo root) + `.GOV/roles_shared/records/TASK_BOARD.md` + work packets. Handshake logger is for milestones/hard bugs when requested.
- **Shared tooling guardrails:** `.GOV/roles_shared/docs/TOOLING_GUARDRAILS.md` (shared tooling memory: short append-only `Do` / `Don't` / `Why` / `Context` notes for all roles).
- **Architecture & debug:** `.GOV/roles_shared/docs/ARCHITECTURE.md` and `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`.
- **Session/runtime law:** `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md` plus the packet-declared external `WP_COMMUNICATION_DIR`.
- **Parallel ownership/worktree law:** `.GOV/roles_shared/docs/ROLE_WORKTREES.md`
- **Canonical command surface:** `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
- **Golden governed workflow examples:** `.GOV/roles_shared/docs/GOVERNED_WORKFLOW_EXAMPLES.md`

## AI Agent Workflow (Mandatory for AI-Autonomous Operation)

**[CX-503, CX-580-623]** This repository is designed for AI-autonomous software engineering. Human users may not have coding expertise and rely on deterministic workflow enforcement.

**Two agent roles:**
1. **Orchestrator** â€” Creates work packets, delegates work, manages workflow
2. **Coder/Debugger** â€” Implements work per work packet scope

**Mandatory protocols:**
- **Orchestrators:** Read `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` before delegating
- **Coders:** Read `.GOV/roles/coder/CODER_PROTOCOL.md` before writing any code
- **Validators:** Read `.GOV/roles/validator/VALIDATOR_PROTOCOL.md` before reviewing, validating, or merging

**Workflow enforcement commands:**
```bash
# Orchestrator: Create work packet from template
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
- Allowed scope (planned diff must be strictly limited to these governance surfaces): `/.GOV/**`, `/.github/**`, `/justfile`, `/.GOV/codex/Handshake_Codex_v1.4.md`, `/AGENTS.md`
- Verification: `just gov-check`
- If any product path is touched (`/src/`, `/app/`, `/tests/`): STOP and require a WP + Gate 0/1 (`just pre-work WP-{ID}` / `just post-work WP-{ID}`)
- Use `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md` for the no-WP recordkeeping flow.
- Governance-maintenance records:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
  - `.GOV/Audits/**` with stable `AUDIT_ID` and, for smoketest reviews, `SMOKETEST_REVIEW_ID`
- Governance-maintenance templates:
  - `.GOV/templates/REPO_GOVERNANCE_TASK_ITEM_TEMPLATE.md`
  - `.GOV/templates/REPO_GOVERNANCE_CHANGELOG_TEMPLATE.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`

**Gate 0 (Pre-Work):** work packet MUST exist and pass `just pre-work WP-{ID}` before implementation starts. If blocked, STOP and request help.

**Gate 1 (Post-Work):** All validation MUST pass `just post-work WP-{ID}` before commit. If blocked, fix issues and re-run.

**Gate visibility (chat UX):** when a gate runs (or blocks), paste the verbatim output and immediately follow with a short phase/status + copy/paste next commands (see role protocols).

**See:** `.GOV/roles_shared/docs/QUALITY_GATE.md` for Gate 0 and Gate 1 requirements.

Quick reference:
- `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md` (drive-agnostic role workflow + operator UX)
- `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md` (live `just` surface by workflow family)
- `.GOV/roles_shared/docs/GOVERNED_WORKFLOW_EXAMPLES.md` (end-to-end governed examples)

## Repo map (open in an editor and `rg`)
- `app/` â€” React + Tauri frontend; UI components live under `app/src/`.
- `app/src-tauri/` â€” Tauri shell; spawns `handshake_core` from `src/backend/handshake_core`.
- `src/backend/handshake_core/` â€” Rust backend crate (API, data, logging).
- `src/shared/` â€” placeholder for cross-stack types/contracts (none defined yet).
- `tests/` â€” top-level test harness placeholder.
- `.GOV/roles_shared/scripts/` â€” shared session, topology, WP, proof, debt, and dev-helper scripts.
- `.GOV/roles_shared/checks/` â€” shared governance and repo checks.
- `.GOV/roles/<role>/{scripts,checks}/` â€” role-owned execution helpers and role-specific checks.
- `.GOV/roles_shared/scripts/hooks/` â€” git hook plumbing only.
- `justfile` â€” operator-facing governance entrypoints that wrap the live role/shared scripts and checks.
- `data/` â€” runtime artifacts; backend logs are written to `data/logs/handshake_core.log`.
- `.GOV/` â€” canonical governance/docs surface.
- `.GOV/operator/` â€” operator-private notes, drafts, and diaries; non-authoritative unless the Operator explicitly designates a specific file for the current task.
- `log_archive/` â€” historical logger drops.
- `.GOV/roles_shared/docs/OWNERSHIP.md` â€” path/area owners for routing reviews.
- Root files: `Handshake_Master_Spec_v*.md`, `.GOV/codex/Handshake_Codex_v1.4.md`, `Handshake_logger_*`, phase/plan docs.
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`, `.GOV/roles/coder/CODER_PROTOCOL.md`, and `.GOV/roles/validator/VALIDATOR_PROTOCOL.md` â€” AI role workflow protocols.

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
git config core.hooksPath .GOV/roles_shared/scripts/hooks
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

For work packets: include scope, expected behavior, in-scope paths, DONE_MEANS, BOOTSTRAP block (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP), and these commands.

CI expectation: run `just validate`; manual validator review is required for MEDIUM/HIGH risk work.

## Bug triage map (jump into RUNBOOK_DEBUG)
- UI/frontend: see `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md#ui-and-shell` (app React + Tauri window lifecycle).
- Backend/API/logic: see `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md#backend-api-and-logic` (Rust `api/*.rs`, models, logging).
- IPC / orchestrator (Tauri â†” Rust core): see `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md#ipc-tauri-bridge` (`app/src-tauri/src/lib.rs` spawn + commands).
- Data/migrations/storage: see `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md#data-storage-and-migrations` (`migrations/`, SQLite, RDD model).

## More context
- Architecture table: `.GOV/roles_shared/docs/ARCHITECTURE.md`
- Debug runbook: `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`
- Current spec + governance: `.GOV/spec/SPEC_CURRENT.md`
- Quality gate (risk tiers + required checks): `.GOV/roles_shared/docs/QUALITY_GATE.md`
- work packet template: `.GOV/templates/TASK_PACKET_TEMPLATE.md`
- Workflow template for reuse: `.GOV/templates/AI_WORKFLOW_TEMPLATE.md`
