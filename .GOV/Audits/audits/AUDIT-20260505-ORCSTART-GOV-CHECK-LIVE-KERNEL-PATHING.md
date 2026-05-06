# AUDIT-20260505-ORCSTART-GOV-CHECK-LIVE-KERNEL-PATHING

- AUDIT_ID: `AUDIT-20260505-ORCSTART-GOV-CHECK-LIVE-KERNEL-PATHING`
- STATUS: APPLIED
- DATE: 2026-05-05
- SCOPE: repo governance startup and `gov-check` path resolution
- DRIVER: `orcstart.cmd` startup failed because `gov-check` launched `repo-governance-board-check.mjs` with `HANDSHAKE_ACTIVE_REPO_ROOT=handshake_main` and `HANDSHAKE_GOV_ROOT=wt-gov-kernel/.GOV`, but the board check resolved `.GOV` path references against the active product root instead of the injected live governance root.

## Finding

`gov-check` intentionally preserves canonical product-root context while running governance subprocesses from the live kernel. The board check did not honor that split: it read governance records and checked `.GOV/*.md` references relative to `HANDSHAKE_ACTIVE_REPO_ROOT`. That made `orcstart` fail when the live kernel contained `.GOV/operator/docs_local/Handshake_Role_Startup_Prompts.md` but the `handshake_main/.GOV` backup did not.

## Change

- `repo-governance-board-check.mjs` now resolves governance record files through `HANDSHAKE_GOV_ROOT` when it is present.
- `repo-governance-board-lib.mjs` now validates `.GOV/` path references against an injected `governanceRoot`.
- `repo-governance-board-lib.test.mjs` covers the split-root case where product root and live governance root differ.

## Verification

- `node --test .GOV/roles_shared/tests/repo-governance-board-lib.test.mjs`
- split-root `repo-governance-board-check.mjs` invocation with `HANDSHAKE_ACTIVE_REPO_ROOT=handshake_main` and `HANDSHAKE_GOV_ROOT=wt-gov-kernel/.GOV`
- `just gov-check`
- `.\\orcstart.cmd`

## Outcome

Orchestrator startup can rely on live kernel governance files during `gov-check` without being blocked by stale backup content in `handshake_main/.GOV`.
