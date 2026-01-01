# AI_WORKFLOW_TEMPLATE (Handshake-derived)

Purpose: capture the exact governance + workflow structure we implemented today so it can be reused in future repos or embedded as a template for local/cloud model workspaces.

This document is intended to be copied into other projects as a starting point. It is not a replacement for a project-specific codex or master spec.

## What we did (summary)
- Created a canonical navigation pack in `docs/` so any model can orient fast.
- Added an explicit spec pointer (`docs/SPEC_CURRENT.md`) and a check to prevent drift.
- Established a debug runbook with a first-5-minutes flow and CI failure triage.
- Added ownership + agent registry so reviews and traceability have a target.
- Introduced a Quality Gate with risk tiers and required validation commands.
- Added scaffolding scripts and enforcement checks to reduce structure drift.
- Standardized manual validator review as the required review artifact.

## Why we did it (rationale)
- Determinism: reduce guesswork about where to look and how to act.
- Traceability: make it easy to track why a change happened months later.
- Error reduction: enforce architecture rules (no direct fetch, no println, etc.).
- Speed: consistent commands and templates reduce repeated setup.
- Debuggability: stable log anchors and runbooks shorten incident triage.

## Canonical inputs and precedence (template)
1) `docs/SPEC_CURRENT.md` (points to current master spec)
2) Codex (repo root)
3) Task Board (`docs/TASK_BOARD.md`) + task packet for the WP
4) Logger (optional; milestones/hard bugs only, root or `log_archive/`)
5) ADRs (`docs/adr/`)
6) Past specs/logs (`docs/PAST_WORK_INDEX.md`)

## Required navigation pack (copy these)
| File | Purpose | Why it matters |
| --- | --- | --- |
| `docs/START_HERE.md` | Entry point + commands | Fast orientation for new models |
| `docs/SPEC_CURRENT.md` | Canonical spec pointer | Prevents spec drift |
| `docs/ARCHITECTURE.md` | Module map + allowed deps | Avoids architectural entropy |
| `docs/RUNBOOK_DEBUG.md` | Debug flow + log map | Consistent incident handling |
| `docs/PAST_WORK_INDEX.md` | Links to old work | Prevents archaeology guesswork |
| `docs/QUALITY_GATE.md` | Risk tiers + required checks | Sets minimum hygiene |
| `docs/templates/TASK_PACKET_TEMPLATE.md` | Standard work packet | Keeps scope/validation consistent |
| `docs/OWNERSHIP.md` | Review routing | Clear accountability |
| `docs/agents/AGENT_REGISTRY.md` | Agent IDs + roles | Traceability for AI work |

## Roles (template)
- Orchestrator: builds task packets; may not have repo access.
- Coder: implements changes; runs local checks; updates docs if needed.
- Debugger: triages issues; uses `RUNBOOK_DEBUG`.
- Validator: performs manual evidence-based review against codex/spec.
- Owner/Reviewer: required review sign-off per `OWNERSHIP.md`.

## Task lifecycle (deterministic flow)
1) Orchestrator produces a task packet using `docs/templates/TASK_PACKET_TEMPLATE.md`.
2) Coder reads `docs/START_HERE.md` + `docs/SPEC_CURRENT.md`.
3) Coder classifies task: DEBUG / FEATURE / REVIEW / REFACTOR / HYGIENE.
4) Coder reads `docs/ARCHITECTURE.md` or `docs/RUNBOOK_DEBUG.md` based on type.
5) Implement change using scaffolds if adding components/endpoints.
6) Run required commands from `docs/QUALITY_GATE.md`.
7) Validator performs manual review and records evidence in the packet or validation report.
8) Update `docs/ARCHITECTURE.md` or `docs/RUNBOOK_DEBUG.md` if new entrypoints or repeatable failures were added.
9) Reviewer validates against codex + required checks.

## Commands (single source)
Keep the authoritative commands in `docs/START_HERE.md` and the task packet. Standard set:
- `just validate` (docs check + lint/tests + depcruise + fmt/clippy + deny)
- `just codex-check`
- `just scaffold-check`

If `just` is unavailable, run the explicit commands directly.

## Scaffolding (structure enforcement)
Use scaffolds for new components/endpoints to avoid drift:
- `just new-react-component <ComponentName>`
- `just new-api-endpoint <endpoint_name>`
- `just scaffold-check` to verify output

## Manual review (required)
Validator performs a manual evidence-based review against the codex/spec and records a PASS/FAIL verdict with evidence mapping.

## Git hook (optional but recommended)
Enable a pre-commit hook for local hygiene checks:
```
git config core.hooksPath scripts/hooks
```

## Validation and enforcement (defaults)
These checks are designed to run in CI or locally:
- `docs-check`: ensures navigation pack exists.
- `codex-check`: disallow direct `fetch(` outside API layer; disallow `println!/eprintln!` in backend; ensure SPEC_CURRENT points to latest spec; enforce TODO tagging.
- `depcruise`: frontend layer boundaries.
- `cargo-deny`: backend dependency audit.
- `gitleaks`: secret scanning.

## Logging and debug anchors
Use stable error tags like `HSK-####` for repeatable failures.
Add those tags to `docs/RUNBOOK_DEBUG.md` with entrypoints and triage notes.
Task Board + task packet act as the micro-log; the Handshake logger is for milestones/hard bugs when requested.

## Repository layout conventions (template)
- `/docs/` is canonical operational guidance.
- `/docs_local/` is staging/legacy and non-binding.
- `/log_archive/` stores historical loggers.
- `/.claude/` stores Claude Code instructions (optional but documented if present).

## How to reuse this template in a new repo
1) Copy the navigation pack files listed above into the new repo.
2) Create a codex and point `docs/SPEC_CURRENT.md` to the master spec.
3) Populate `docs/ARCHITECTURE.md` with real entrypoints.
4) Add `docs/RUNBOOK_DEBUG.md` with log locations and first-5-minutes flow.
5) Add scaffolding scripts and wire `justfile` targets.
6) Require manual validator review and evidence mapping.
7) Add CI jobs for lint/tests/depcruise/deny/gitleaks as available.
8) Add ownership and agent registry rows for the team/roles.

## Optional extensions
- Use optional automated review tooling as a secondary reviewer for high-risk changes.
- Add custom lint rules or architecture tests for deeper enforcement.
- Add a `KNOWN_DEVIATIONS` section in the codex for intentional layout drift.
