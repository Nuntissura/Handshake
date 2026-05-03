# Orcstart Authority Contract Audit

AUDIT_ID: AUDIT-20260504-ORCSTART-AUTHORITY-CONTRACT
STATUS: CLOSED
DATE: 2026-05-04
OWNER: ORCHESTRATOR
SCOPE: Repo Governance

## Driver

The Operator asked for `orcstart.cmd` to inject the cheat-sheet Orchestrator prompt as a repo-governing rule set and make the assistant read the Codex, `AGENTS.md`, and Orchestrator protocol as a contract while remaining model agnostic.

## Finding

The existing `orcstart` launcher already extracted the `ORCHESTRATOR - Startup Prompt` block from `.GOV/operator/docs_local/Handshake_Role_Startup_Prompts.md` and ran `just orchestrator-startup`. The weak point was contractual clarity: the injected prompt allowed startup output to act as compact authority context and did not require explicit post-startup reads of the three named authority files.

## Decision

- Extend the existing `orcstart` launcher instead of adding another public command.
- Keep the launcher model/provider agnostic: it prints deterministic prompt, startup, contract, and file context; it does not launch Codex, Claude, ChatGPT, or another model process.
- Print a `REPO GOVERNING RULE SET` contract before the role prompt.
- After startup, inject the required authority file contents for `../handshake_main/AGENTS.md`, `.GOV/codex/Handshake_Codex_v1.4.md`, and `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`.
- Update the cheat-sheet Orchestrator prompt so the source prompt itself requires full post-startup authority reads before claiming startup completion.

## Primary Surfaces

- `.GOV/operator/scripts/orcstart.ps1`
- `.GOV/operator/scripts/orcstart.prompt.txt`
- `.GOV/operator/docs_local/Handshake_Role_Startup_Prompts.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

## Verification

- `.\orcstart.cmd --no-startup --no-authority-files`
- `.\orcstart.cmd --no-startup` marker check for all three `AUTHORITY_FILE_BEGIN` / `AUTHORITY_FILE_END` pairs and no `[orcstart] MISSING` authority-file errors
- `git diff --check`
- `just gov-check`
