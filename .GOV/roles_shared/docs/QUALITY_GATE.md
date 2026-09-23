# QUALITY_GATE

Purpose: reduce coding errors by standard checks and clear risk tiers.

## Gate 0: Pre-Work Validation (AI Autonomy - Mandatory)

**[CX-620, CX-587]** Before any implementation work starts, Gate 0 MUST pass.

**For Orchestrator Agents:**
- work packet MUST exist at the authoritative resolved packet path:
  logical `.GOV/work_packets/WP-{ID}/packet.md`; current physical storage `.GOV/task_packets/WP-{ID}/packet.md`; legacy flat compatibility `.GOV/task_packets/WP-{ID}.md`
- All work packet fields MUST be filled (no `{placeholders}`)

**For Coder Agents:**
- work packet MUST be verified before writing any code
- If no packet found, work MUST be BLOCKED immediately
- Bootstrap protocol MUST be followed (read START_HERE, SPEC_CURRENT, packet)
- BOOTSTRAP block MUST be output before first code change

**Enforcement:** Gate 0 is checked by reading the artifact (check script deleted 2026-09-23).

**Governance-only maintenance (no WP required) [CX-111]:**
- If the planned diff is strictly limited to governance surface files (`/.GOV/**`, `/.github/**`, `/.GOV/codex/Handshake_Codex_v1.4.md`, `/AGENTS.md`), a Work Packet is not required.
- Verification for governance-only changes: checked by reading the artifact (check script deleted 2026-09-23).
- If any product path is touched (`/src/`, `/app/`, `/tests/`), STOP and require a WP + Gate 0/1.

**Why:** For AI-autonomous operation, the workflow requires deterministic enforcement. Human users may not have coding expertise and rely on these gates to ensure correctness.

## Risk tiers
| Tier | Use when | Required checks | Review |
| --- | --- | --- | --- |
| LOW | Docs-only or comments; no behavior change | Read the touched docs (check script deleted 2026-09-23) | Optional owner review |
| MEDIUM | Code change within one module; no schema/IPC changes | explicit WP `TEST_PLAN` commands | Owner review required |
| HIGH | Cross-module, IPC, migrations, auth/security, dependency updates, perf-critical | explicit WP `TEST_PLAN` commands + manual test steps | Two reviewers (owner + secondary) |

If uncertain, choose the higher tier.

## work packet fields (required)
- RISK_TIER (LOW/MEDIUM/HIGH)
- TEST_PLAN (commands + manual steps, or "None" with reason)
- ROLLBACK_HINT (how to revert if needed)
- DONE_MEANS (what must be true to accept)

## Definition of done (minimum)
- Required commands run (or recorded why not).
- Any new error codes/tags documented in `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`.
- New flags/toggles documented in `.GOV/roles_shared/docs/ARCHITECTURE.md`.
- Targeted test added for logic changes, or explicit reason recorded.
- Manual validator review completed and recorded (status + evidence mapping); no automated review required.

Product hygiene should be declared explicitly in the WP `TEST_PLAN`, typically combining the required frontend/backend commands such as `pnpm -C app run lint`, `pnpm -C app test`, and `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`.

## Gate 1: Post-Work Validation (AI Autonomy - Mandatory)

**[CX-623, CX-651]** Before requesting commit, Gate 1 MUST pass.

**Required:**
- All TEST_PLAN commands MUST have been run
- Validation results MUST be documented in the work packet (logger only if explicitly requested)
- Git status MUST show changes (work actually done)
- For MEDIUM/HIGH: Manual validator review must be complete before marking Done
- work packet MUST capture current status/result

**Enforcement:** Gate 1 is checked by reading the artifact (check script deleted 2026-09-23).

## Self-review checklist (required)
1) Diff scan: every line is necessary for the task; no drive-by changes.
2) Placement: files and functions live in the correct module (see `.GOV/roles_shared/docs/ARCHITECTURE.md`).
3) Errors/observability: new repeatable errors have `HSK-####` tags and `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` updates.
4) Tests: at least one targeted test for logic changes (or a written reason).
5) Docs drift: update `.GOV/roles_shared/docs/START_HERE.md`, `.GOV/roles_shared/docs/ARCHITECTURE.md`, `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` if behavior/entrypoints changed.
