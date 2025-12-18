# QUALITY_GATE

Purpose: reduce coding errors by standard checks and clear risk tiers.

## Risk tiers
| Tier | Use when | Required checks | Review |
| --- | --- | --- | --- |
| LOW | Docs-only or comments; no behavior change | `just docs-check` (if docs touched) | Optional owner review; AI review optional |
| MEDIUM | Code change within one module; no schema/IPC changes | `just validate` (or record why not) | Owner review required + AI review (`just ai-review`) |
| HIGH | Cross-module, IPC, migrations, auth/security, dependency updates, perf-critical | `just validate` + manual test steps | Two reviewers (owner + secondary) + AI review (`just ai-review`) |

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
- AI review result recorded (PASS or WARN acknowledged) by attaching `ai_review.md` from `just ai-review`. BLOCK must be resolved.

`just validate` runs: `just docs-check`, `just codex-check`, `pnpm -C app run lint`, `pnpm -C app test`, `pnpm -C app run depcruise`, `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`, `cargo deny check advisories licenses bans sources`.

AI review runs locally via `just ai-review` using the `gemini` CLI and the output `ai_review.md` must be attached to the task packet/logger.

## Self-review checklist (required)
1) Diff scan: every line is necessary for the task; no drive-by changes.
2) Placement: files and functions live in the correct module (see `docs/ARCHITECTURE.md`).
3) Errors/observability: new repeatable errors have `HSK-####` tags and `docs/RUNBOOK_DEBUG.md` updates.
4) Tests: at least one targeted test for logic changes (or a written reason).
5) Docs drift: update `docs/START_HERE.md`, `docs/ARCHITECTURE.md`, `docs/RUNBOOK_DEBUG.md` if behavior/entrypoints changed.

Scaffolding: for new components or API endpoints, prefer `just new-react-component <Name>` or `just new-api-endpoint <name>` to keep structure consistent.
For MEDIUM/HIGH tasks adding new components or endpoints, scaffolds are required unless explicitly waived; record the reason in the task packet and run `just scaffold-check` before merge.
