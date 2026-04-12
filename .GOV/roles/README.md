# Role Bundles

This README is navigational only.
Authoritative role folder law lives in `.GOV/codex/Handshake_Codex_v1.4.md` plus the active role protocol for the role you are working in.

Each role now has a local bundle README that groups:

- protocol and rubric files
- role-specific state and gate files
- relevant `just` commands
- relevant scripts/checks
- shared files that the role relies on most

Shared artifacts used by more than one role belong under `.GOV/roles_shared/`, not a role-local bundle.

Do not place new historical/reference studies at the role root. Put them under the role's `docs/` directory or under `.GOV/reference/` if they are shared historical material.

Open one of:

- `.GOV/roles/orchestrator/README.md`
- `.GOV/roles/activation_manager/README.md`
- `.GOV/roles/coder/README.md`
- `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md` — per-MT boundary enforcement, scope containment, code review (orchestrator-managed workflow)
- `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md` — whole-WP judgment, verdict, merge authority (orchestrator-managed workflow)
- `.GOV/roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md` — manual relay workflow coordination role
- `.GOV/roles/validator/README.md` — shared validation foundation protocol
- `.GOV/roles/memory_manager/README.md`

Shared cross-role state lives in `.GOV/roles_shared/README.md`.
