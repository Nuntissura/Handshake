# Deprecation Sunset Plan

Active compatibility surfaces must be recorded here until removed.

## ENTRY
- LEGACY_SURFACE: `WINDOWS_TERMINAL`
- STATUS: `ACTIVE_COMPAT`
- CANONICAL_REPLACEMENT: `SYSTEM_TERMINAL`
- OWNER: `ORCHESTRATOR`
- NEW_DEPENDENCIES_ALLOWED: `NO`
- WHY_THIS_EXISTS: `older packets, launch notes, and human habits still use the old host token while the governed launch path has already standardized on SYSTEM_TERMINAL`
- SUPERSEDED_BY: `session-policy normalization + launch-cli-session guidance that emits SYSTEM_TERMINAL as the canonical host`
- DELETION_CONDITION: `no active packet, stub, protocol example, or runtime helper emits or requires WINDOWS_TERMINAL anymore`
- SUNSET_TRIGGER: `all active packets/stubs/protocol examples and governed runtime defaults use SYSTEM_TERMINAL only, and session-policy normalization no longer needs the old token`
- REMOVAL_ACTION: `remove legacy alias acceptance from session-policy and launch tooling`

## ENTRY
- LEGACY_SURFACE: `.GOV/roles_shared/runtime/*`
- STATUS: `ACTIVE_COMPAT`
- CANONICAL_REPLACEMENT: `../gov_runtime/roles_shared/*`
- OWNER: `ORCHESTRATOR`
- NEW_DEPENDENCIES_ALLOWED: `NO`
- WHY_THIS_EXISTS: `historical packets, migration checks, and residue detection still reference or guard against repo-local runtime authority while external gov_runtime canonicalization finishes`
- SUPERSEDED_BY: `runtime-placement-check + migration-path-truth-check + external gov_runtime authority paths in runtime-paths/session-policy`
- DELETION_CONDITION: `no live launch/control/WP tooling resolves repo-local runtime authority and no active migration check needs repo-local compatibility branches beyond archived reference material`
- SUNSET_TRIGGER: `all active packets/docs/checks resolve session/control/WP runtime paths only through external gov_runtime authority and the repo-local runtime constants can be removed without breaking replay or audits`
- REMOVAL_ACTION: `delete repo-local runtime compatibility constants and migration allowances for session/control/WP authority surfaces`

## ENTRY
- LEGACY_SURFACE: `.GOV/roles/validator/VALIDATOR_GATES.json`
- STATUS: `REMOVED`
- CANONICAL_REPLACEMENT: `.GOV/roles_shared/runtime/validator_gates/{WP_ID}.json`
- OWNER: `VALIDATOR`
- NEW_DEPENDENCIES_ALLOWED: `NO`
- WHY_THIS_EXISTS: `older validator sessions wrote one shared gate file, which made active authority and historical evidence hard to separate`
- SUPERSEDED_BY: `.GOV/roles_shared/runtime/validator_gates/{WP_ID}.json` plus the legacy archive at .GOV/reference/legacy/validator/VALIDATOR_GATES.json`
- DELETION_CONDITION: `all active validator tooling reads per-WP gate files only and the old shared file remains historical reference material only`
- SUNSET_TRIGGER: `all active validator tooling reads per-WP gate files only and no live authority surface requires the legacy archive path outside explicitly marked reference material`
- REMOVAL_ACTION: `legacy root archive removed; preserved historical copy at .GOV/reference/legacy/validator/VALIDATOR_GATES.json and removed active workflow dependencies on the old root path`
