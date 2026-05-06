# Orcstart Nonzero Startup Warning Audit

AUDIT_ID: AUDIT-20260506-ORCSTART-NONZERO-STARTUP-WARNING
STATUS: CLOSED
DATE: 2026-05-06
OWNER: ORCHESTRATOR
SCOPE: Repo Governance

## Driver

The Operator observed that `orcstart.cmd` returned exit code `1` when `just orchestrator-startup` failed deterministic checks, even though the launcher had already injected the role prompt, repo-governing contract, and authority-file contents. That nonzero launcher exit caused the assistant to treat startup as incomplete instead of reading and obeying the injected authority.

## Finding

`orcstart.ps1` stored the `just orchestrator-startup` exit code in `$script:startupExitCode` and reused that value as the launcher exit code. This conflated two different outcomes:

- startup-state checks reported debt or failed governance checks
- authority injection failed because prompt or required authority files were missing

Only the second class should block role startup. The first class should remain visible as startup state context.

## Decision

- Keep `orcstart.cmd` as the single operator-facing startup launcher.
- Do not add a second wrapper or bypass `just orchestrator-startup`.
- Capture `just orchestrator-startup` output while streaming it.
- If `just orchestrator-startup` exits nonzero, print `STARTUP WARNING: FIRST COMMAND NONZERO`, include likely parsed causes, and continue authority-file injection.
- Return launcher exit code `0` when prompt/authority injection succeeds, even if the startup recipe emitted a warning.
- Return launcher exit code `1` only when prompt extraction or required authority-file injection fails.
- Update the injected startup prompt contract so `AUTHORITY_READ_CONTRACT` explicitly applies after either FIRST COMMAND completion or STARTUP WARNING.

## Primary Surfaces

- `.GOV/operator/scripts/orcstart.ps1`
- `.GOV/operator/scripts/orcstart.prompt.txt`
- `.GOV/operator/docs_local/Handshake_Role_Startup_Prompts.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

## Verification

- `.\orcstart.cmd --help`
- `.\orcstart.cmd --print`
- `.\orcstart.cmd --brief`
- `git diff --check`
- `just gov-check`
