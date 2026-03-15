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
  - fixed role-local subfolders
  - legacy root state may exist during migration, but new role-owned state belongs under `runtime/`
- `docs/`
  - role-local non-authoritative docs such as rubrics, roadmaps, priorities, and protocol gap analyses
- `runtime/`
  - role-owned machine state only
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

Shared artifacts used by more than one role belong under `.GOV/roles_shared/`, not a role-local bundle.

Do not place new historical/reference studies at the role root. Put them under the role's `docs/` directory or under `.GOV/reference/` if they are shared historical material.

Open one of:

- `.GOV/roles/orchestrator/README.md`
- `.GOV/roles/coder/README.md`
- `.GOV/roles/validator/README.md`

Shared cross-role state lives in `.GOV/roles_shared/README.md`.
