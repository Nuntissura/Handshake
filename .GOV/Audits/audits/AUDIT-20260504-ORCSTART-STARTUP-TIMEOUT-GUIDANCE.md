# Orcstart Startup Timeout Guidance Audit

AUDIT_ID: AUDIT-20260504-ORCSTART-STARTUP-TIMEOUT-GUIDANCE
STATUS: CLOSED
DATE: 2026-05-04
OWNER: ORCHESTRATOR
SCOPE: Repo Governance

## Driver

The Operator observed that `orcstart` startup hit the caller shell timeout and asked to raise the visible timeout guidance: startup can take a while, use at least a 10-minute timeout, and preferably longer under host load.

## Finding

The Orchestrator startup prompt already named `600000` ms / 10 minutes, but the launcher bootstrap and help text did not foreground the timeout before the long `just orchestrator-startup` phase. In this run, startup completed successfully only when rerun with `600000` ms and took about 406 seconds, leaving little margin for a 10-minute ceiling on a loaded host.

## Decision

- Keep `orcstart.cmd` model/provider agnostic and continue using the existing public launcher.
- Do not add a second startup command or wrapper.
- Print explicit timeout guidance from `orcstart.ps1` before startup work begins and in `--help`.
- Treat `600000` ms / 10 minutes as the minimum and `1200000` ms / 20 minutes as the recommended timeout on this host under load.
- Update the operator startup prompt source and prompt mirror so future assistant invocations use the longer recommended shell timeout.

## Primary Surfaces

- `.GOV/operator/scripts/orcstart.ps1`
- `.GOV/operator/scripts/orcstart.prompt.txt`
- `.GOV/operator/docs_local/Handshake_Role_Startup_Prompts.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

## Verification

- `.\orcstart.cmd --help`
- `.\orcstart.cmd --print`
- `git diff --check`
- `just gov-check`
