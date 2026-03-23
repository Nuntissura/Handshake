# Validator Gate State (Archive-Only Reference)

This repo-local directory is archive-only reference material.

Live validator gate state is stored per WP under the external governance runtime root:
- `../gov_runtime/roles_shared/validator_gates/{WP_ID}.json`

New validator sessions should write only to the external runtime path above.

Why:
- avoids cross-WP merge conflicts that occur when multiple validations append to a single global JSON ledger
- keeps live machine-written validator runtime state out of the repo worktree

Legacy:
- `.GOV/reference/legacy/validator/VALIDATOR_GATES.json` remains a read-only archive for older sessions
- `.GOV/roles_shared/runtime/validator_gates/` remains a repo-local archive/reference surface and must not receive new live writes
