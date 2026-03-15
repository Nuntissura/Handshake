# Role Bundles

Each role now has a local bundle README that groups:

- protocol and rubric files
- role-specific state and gate files
- relevant `just` commands
- relevant scripts/checks
- shared files that the role relies on most

Folder law:

- role root:
  - role protocol
  - role README
  - stable role state files
  - `agentic/` only when that role has an approved agentic protocol
- `docs/`
  - role-local non-authoritative docs such as rubrics, roadmaps, priorities, and protocol gap analyses
- `scripts/`
  - role-owned executable entrypoints
- `scripts/lib/`
  - helper libraries used only by that role's scripts/checks
- `checks/`
  - role-owned enforcement/validation entrypoints
- `tests/`
  - role-owned governance tests
- `fixtures/`
  - role-owned test fixtures/golden inputs

Do not place new historical/reference studies at the role root. Put them under the role's `docs/` directory or under `.GOV/reference/` if they are shared historical material.

See:

- `.GOV/roles/STRUCTURE_RULES.md`
- `.GOV/roles_shared/STRUCTURE_RULES.md`

Open one of:

- `.GOV/roles/orchestrator/README.md`
- `.GOV/roles/coder/README.md`
- `.GOV/roles/validator/README.md`

Shared cross-role state lives in `.GOV/roles_shared/README.md`.
