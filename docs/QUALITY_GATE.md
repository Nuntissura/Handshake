# QUALITY_GATE

Purpose: reduce coding errors by standard checks and clear risk tiers.

## Gate 0: Pre-Work Validation (AI Autonomy - Mandatory)

**[CX-620, CX-587]** Before any implementation work starts, Gate 0 MUST pass.

**For Orchestrator Agents:**
- Task packet MUST exist in `docs/task_packets/WP-{ID}.md`
- All task packet fields MUST be filled (no `{placeholders}`)
- Verification: `just pre-work WP-{ID}` MUST pass

**For Coder Agents:**
- Task packet MUST be verified before writing any code
- If no packet found, work MUST be BLOCKED immediately
- Bootstrap protocol MUST be followed (read START_HERE, SPEC_CURRENT, packet)
- BOOTSTRAP block MUST be output before first code change

**Enforcement:** Gate 0 is automated via validation scripts. Failure exits 1 and blocks work.

**Why:** For AI-autonomous operation, the workflow requires deterministic enforcement. Human users may not have coding expertise and rely on these gates to ensure correctness.

## Risk tiers
| Tier | Use when | Required checks | Review |
| --- | --- | --- | --- |
| LOW | Docs-only or comments; no behavior change | `just docs-check` (if docs touched) | Optional owner review |
| MEDIUM | Code change within one module; no schema/IPC changes | `just validate` (or record why not) | Owner review required |
| HIGH | Cross-module, IPC, migrations, auth/security, dependency updates, perf-critical | `just validate` + manual test steps | Two reviewers (owner + secondary) |

If uncertain, choose the higher tier.

## Task packet fields (required)
- RISK_TIER (LOW/MEDIUM/HIGH)
- TEST_PLAN (commands + manual steps, or "None" with reason)
- ROLLBACK_HINT (how to revert if needed)
- DONE_MEANS (what must be true to accept)

## Definition of done (minimum)
- Required commands run (or recorded why not).
- Any new error codes/tags documented in `docs/RUNBOOK_DEBUG.md`.
- New flags/toggles documented in `docs/ARCHITECTURE.md`.
- Targeted test added for logic changes, or explicit reason recorded.
- Manual validator review completed and recorded (status + evidence mapping); no automated review required.

`just validate` runs: `just docs-check`, `just codex-check`, `pnpm -C app run lint`, `pnpm -C app test`, `pnpm -C app run depcruise`, `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`, `cargo deny check advisories licenses bans sources`.

## Gate 1: Post-Work Validation (AI Autonomy - Mandatory)

**[CX-623, CX-651]** Before requesting commit, Gate 1 MUST pass.

**Required:**
- All TEST_PLAN commands MUST have been run
- Validation results MUST be documented in the task packet (logger only if explicitly requested)
- Git status MUST show changes (work actually done)
- For MEDIUM/HIGH: Manual validator review must be complete before marking Done
- Task packet MUST capture current status/result
- Verification: `just post-work WP-{ID}` MUST pass

**Enforcement:** Gate 1 is automated via validation scripts. Failure exits 1 and blocks commit.

**Full workflow validation:**
```bash
just validate-workflow WP-{ID}  # Runs pre-work, validate, post-work
```

## Self-review checklist (required)
1) Diff scan: every line is necessary for the task; no drive-by changes.
2) Placement: files and functions live in the correct module (see `docs/ARCHITECTURE.md`).
3) Errors/observability: new repeatable errors have `HSK-####` tags and `docs/RUNBOOK_DEBUG.md` updates.
4) Tests: at least one targeted test for logic changes (or a written reason).
5) Docs drift: update `docs/START_HERE.md`, `docs/ARCHITECTURE.md`, `docs/RUNBOOK_DEBUG.md` if behavior/entrypoints changed.

Scaffolding: for new components or API endpoints, prefer `just new-react-component <Name>` or `just new-api-endpoint <name>` to keep structure consistent.
For MEDIUM/HIGH tasks adding new components or endpoints, scaffolds are required unless explicitly waived; record the reason in the task packet and run `just scaffold-check` before merge.
