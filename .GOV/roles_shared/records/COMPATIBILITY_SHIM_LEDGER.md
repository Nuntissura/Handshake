# Compatibility Shim Ledger

Every active compatibility shim must be recorded here until its sunset trigger is complete.

## SHIM
- SHIM_ID: `GOV-SHIM-WINDOWS-TERMINAL-ALIAS`
- STATUS: `ACTIVE_COMPAT`
- LEGACY_SURFACE: `WINDOWS_TERMINAL`
- SHIM_KIND: `TOKEN_ALIAS`
- OWNER: `ORCHESTRATOR`
- WHY_THIS_EXISTS: `Older packets, operator notes, and launch habits still use WINDOWS_TERMINAL while governed launch policy has standardized on SYSTEM_TERMINAL.`
- SUPERSEDED_BY: `SYSTEM_TERMINAL`
- TRACKED_IN: `.GOV/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md`
- ACTIVE_GUARDRAILS: `.GOV/roles_shared/checks/deprecation-sunset-check.mjs`, `.GOV/roles_shared/checks/session-policy-check.mjs`
- SUNSET_TRIGGER: `All active packets, stubs, docs, and launch helpers use SYSTEM_TERMINAL only.`
- DELETION_CONDITION: `Session-policy normalization and launch tooling no longer need to accept WINDOWS_TERMINAL as an alias.`

## SHIM
- SHIM_ID: `GOV-SHIM-REPO-LOCAL-RUNTIME-COMPAT`
- STATUS: `ACTIVE_COMPAT`
- LEGACY_SURFACE: `.GOV/roles_shared/runtime/*`
- SHIM_KIND: `PATH_COMPAT`
- OWNER: `ORCHESTRATOR`
- WHY_THIS_EXISTS: `Migration residue and historical references still guard against repo-local runtime authority while external gov_runtime remains canonical.`
- SUPERSEDED_BY: `../gov_runtime/roles_shared/*`
- TRACKED_IN: `.GOV/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md`
- ACTIVE_GUARDRAILS: `.GOV/roles_shared/checks/deprecation-sunset-check.mjs`, `.GOV/roles_shared/checks/runtime-placement-check.mjs`, `.GOV/roles_shared/checks/migration-path-truth-check.mjs`
- SUNSET_TRIGGER: `No live launch/control/WP tooling resolves repo-local runtime authority and migration checks no longer need compatibility branches.`
- DELETION_CONDITION: `Repo-local runtime compatibility constants and allowances can be removed without breaking replay or audits.`

## SHIM
- SHIM_ID: `GOV-SHIM-REPO-LOCAL-VALIDATOR-GATES`
- STATUS: `ACTIVE_COMPAT`
- LEGACY_SURFACE: `.GOV/roles_shared/runtime/validator_gates/*`
- SHIM_KIND: `ARCHIVE_PATH_COMPAT`
- OWNER: `VALIDATOR`
- WHY_THIS_EXISTS: `Historical validator gate JSON files still exist for archaeology while live validator gate authority has moved to external gov_runtime.`
- SUPERSEDED_BY: `../gov_runtime/roles_shared/validator_gates/{WP_ID}.json`
- TRACKED_IN: `.GOV/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md`
- ACTIVE_GUARDRAILS: `.GOV/roles_shared/checks/deprecation-sunset-check.mjs`, `.GOV/roles_shared/checks/runtime-placement-check.mjs`, `.GOV/roles_shared/checks/migration-path-truth-check.mjs`
- SUNSET_TRIGGER: `Active validators write only to ../gov_runtime/roles_shared/validator_gates/{WP_ID}.json and repo-local validator gate paths no longer receive live state.`
- DELETION_CONDITION: `Repo-local validator gate compatibility paths are archive-only and can be retired from live tooling.`

## SHIM
- SHIM_ID: `GOV-SHIM-LEGACY-PACKET-LAW-FAMILY`
- STATUS: `ACTIVE_COMPAT`
- LEGACY_SURFACE: `PACKET_FORMAT_VERSION < 2026-04-01 packet family`
- SHIM_KIND: `VALIDATION_RULE_GRANDFATHERING`
- OWNER: `ORCHESTRATOR`
- WHY_THIS_EXISTS: `Older packets predate the explicit data-contract activation/waiver decision and anti-vibe zero-debt closure bundle, so governance checks still need an explicit grandfather branch while those packets remain live or historically referenced.`
- SUPERSEDED_BY: `PACKET_FORMAT_VERSION >= 2026-04-01 packet family with explicit DATA_CONTRACT_DECISION + anti-vibe zero-debt law`
- TRACKED_IN: `.GOV/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md`
- ACTIVE_GUARDRAILS: `.GOV/roles_shared/checks/task-packet-claim-check.mjs`, `.GOV/roles/validator/checks/validator-report-structure-check.mjs`, `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs` (`validator-packet-complete`)
- SUNSET_TRIGGER: `All live packets are created or migrated onto the 2026-04-01+ family and claim/validator checks no longer need grandfather branches.`
- DELETION_CONDITION: `Compatibility branches for the older packet family can be removed without breaking live packets or audit replay.`
