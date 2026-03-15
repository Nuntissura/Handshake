# Deprecation Sunset Plan

Active compatibility surfaces must be recorded here until removed.

## ENTRY
- LEGACY_SURFACE: `WINDOWS_TERMINAL`
- STATUS: `ACTIVE_COMPAT`
- CANONICAL_REPLACEMENT: `SYSTEM_TERMINAL`
- OWNER: `ORCHESTRATOR`
- NEW_DEPENDENCIES_ALLOWED: `NO`
- SUNSET_TRIGGER: `all active packets/stubs/protocol examples and governed runtime defaults use SYSTEM_TERMINAL only, and session-policy normalization no longer needs the old token`
- REMOVAL_ACTION: `remove legacy alias acceptance from session-policy and launch tooling`

## ENTRY
- LEGACY_SURFACE: `.GOV/roles/validator/VALIDATOR_GATES.json`
- STATUS: `REMOVED`
- CANONICAL_REPLACEMENT: `.GOV/roles_shared/runtime/validator_gates/{WP_ID}.json`
- OWNER: `VALIDATOR`
- NEW_DEPENDENCIES_ALLOWED: `NO`
- SUNSET_TRIGGER: `all active validator tooling reads per-WP gate files only and no live authority surface requires the legacy archive path outside explicitly marked reference material`
- REMOVAL_ACTION: `legacy root archive removed; preserved historical copy at .GOV/reference/legacy/validator/VALIDATOR_GATES.json and removed active workflow dependencies on the old root path`
