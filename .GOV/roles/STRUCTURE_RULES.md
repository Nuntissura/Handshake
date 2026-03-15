# Role Structure Rules

This file defines the canonical folder law for `/.GOV/roles/<role>/`.

## Root Of A Role Folder

Allowed at role root:

- `<ROLE>_PROTOCOL.md`
- `README.md`
- stable role state files such as gate/state JSON that are truly role-owned
- `agentic/` only if that role has an explicit agentic protocol

Do not leave roadmaps, gap analyses, rubrics, or historical notes at role root.

## Canonical Subfolders

- `docs/`
  - role-local, non-authoritative documentation
  - examples: rubrics, implementation roadmaps, priorities, protocol scrutiny, gap analysis
- `scripts/`
  - role-owned executable entrypoints
- `scripts/lib/`
  - helper libraries used only by that role's scripts/checks
- `checks/`
  - role-owned enforcement and validation entrypoints
- `tests/`
  - governance tests for that role's scripts/checks
- `fixtures/`
  - role-local golden files, synthetic inputs, and test fixtures

## Placement Rules

- If a file is executable by `just`, CI, or another role, it belongs in `scripts/` or `checks/`, not `docs/`.
- If a file explains, critiques, or plans the role, it belongs in `docs/`.
- If a file is shared by more than one role, it does not belong in a role folder. Move it to `/.GOV/roles_shared/` or `/.GOV/reference/` depending on whether it is active or historical.
