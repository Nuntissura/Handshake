# Role Workflow Quick Reference (Drive-Agnostic + Operator UX)

This doc is a compact index of the role-governed workflow so the Operator can quickly sanity-check:
- what each role is supposed to do,
- which files are authoritative,
- which commands are governance-only vs product-scanning,
- and where drive-specific paths are forbidden.

## Drive-Agnostic Rules (Repo Governance)

- Authority: `Handshake Codex v1.4.md` [CX-109], [CX-110].
- Role worktree layout is defined in `.GOV/roles_shared/ROLE_WORKTREES.md` using placeholders:
  - `<HANDSHAKE_ROOT>` (example: `P:\Handshake`)
  - `<HANDSHAKE_WORKTREES>` = `<HANDSHAKE_ROOT>\Handshake Worktrees`
- WP worktree assignments MUST be recorded as repo-relative paths:
  - `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` `PREPARE.worktree_dir` should be like `../wt-WP-...`
  - Absolute paths (for example `<DRIVE>:\...` or `\\server\share\...`) are forbidden for `worktree_dir` and are blocked by the Orchestrator gate.

## Operator UX (Chat Output Order)

All roles SHOULD follow a strict ordering to avoid interleaving narrative with evidence:
1) `ROLE=...` header line (per Operator instructions)
2) `LIFECYCLE [CX-LIFE-001]`
3) `OPERATOR_ACTION: NONE` (or one explicit decision needed)
4) `STATE: ...` (1 line)
5) Short `FINDINGS:` bullets (2-6 lines max)
6) If a gate command ran: paste `GATE_OUTPUT [CX-GATE-UX-001]` as a single verbatim block
7) Then `GATE_STATUS [CX-GATE-UX-001]` + `NEXT_COMMANDS [CX-GATE-UX-001]` (copy/paste ready)

## Governance vs Product Checks

Governance-only (does not scan `src/` or `app/`):
- `just gov-check`

Product-scanning / product-boundary enforcement:
- `just codex-check` (includes hard boundary checks for `.GOV` references in product code)
- `just product-scan` (alias) / `just validator-scan` (forbidden patterns in product sources)
- `just validate` (full product hygiene: frontend + backend tests, etc.)

## Role: Orchestrator

Authoritative inputs:
- `.GOV/roles_shared/SPEC_CURRENT.md` (current binding spec pointer)
- `.GOV/roles_shared/TASK_BOARD.md`
- `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` (mechanical gate state)

Primary commands:
- `just record-refinement WP-...`
- `just record-signature WP-... <sig>`
- `just worktree-add WP-...`
- `just record-prepare WP-... <Coder-A|Coder-B> [branch] [worktree_dir]`
- `just create-task-packet WP-...`
- `just pre-work WP-...`

## Role: Coder

Primary commands:
- `just pre-work WP-...`
- Implement only within `IN_SCOPE_PATHS`
- Hygiene: `just product-scan`, `just validator-dal-audit`, `just validator-git-hygiene`
- Workflow closure evidence: `just post-work WP-...`

## Role: Validator

Primary commands (per WP validation):
- `just gate-check WP-...`
- `just post-work WP-...`
- `just validator-dal-audit`
- `just validator-git-hygiene`
- `just codex-check` (product boundary enforcement)

Governance-only work:
- `just gov-check`

File-touch map:
- `.GOV/roles_shared/VALIDATOR_FILE_TOUCH_MAP.md`
