# Governance Prevention Ladder

This ledger promotes repeated governance escapes into named prevention assets.

## ESCAPE
- ESCAPE_ID: `GOV-ESCAPE-WORKFLOW-CONTRACT-FIELDS`
- STATUS: `ENFORCED`
- TITLE: `Missing workflow contract field enforcement`
- TRIGGER_AUDIT: `.GOV/Audits/audits/AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT.md`
- FAILURE_SHAPE: `A packet could look active or closeable while workflow-contract fields or direct-review contract fields were weak, missing, or only narratively defended.`
- CANONICAL_CHECKS: `.GOV/roles_shared/checks/task-packet-claim-check.mjs`, `.GOV/roles_shared/checks/wp-communication-health-check.mjs`, `.GOV/roles_shared/checks/computed-policy-gate-check.mjs`
- CANONICAL_ASSETS: `DIRECT_REVIEW_V1`, `HANDOFF_VERDICT_BLOCKING`, `WORKFLOW_VALIDITY`, `COMMUNICATION_HEALTH_GATE`
- PROMOTION_RULE: `If this shape reappears, raise the missing field from claim-check failure to packet-creation hard fail and add a regression fixture at the exact boundary.`
- NEXT_ESCALATION: `Add packet-build-time refusal for any orchestrator-managed packet that omits the required workflow contract tuple.`
- EXIT_CONDITION: `Workflow contract drift cannot clear packet claim checks, direct-review health checks, or validator commit clearance.`

## ESCAPE
- ESCAPE_ID: `GOV-ESCAPE-NESTED-PAYLOAD-VALIDATION`
- STATUS: `SEEDED`
- TITLE: `Shallow nested payload validation`
- TRIGGER_AUDIT: `.GOV/Audits/audits/AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT.md`
- FAILURE_SHAPE: `Outer packet/report shape passed while nested payload fields, structured evidence blocks, or child contract objects were only partially defended.`
- CANONICAL_CHECKS: `.GOV/roles/validator/checks/validator-report-structure-check.mjs`, `.GOV/roles/validator/checks/validator-packet-complete.mjs`, `.GOV/roles_shared/checks/computed-policy-gate-check.mjs`
- CANONICAL_ASSETS: `VALIDATION_REPORTS`, `CLAUSE_CLOSURE_MATRIX`, `SPEC_CLAUSE_MAP`, `NEGATIVE_PROOF`
- PROMOTION_RULE: `If nested payload drift reappears, extract the weak nested block into a named shared schema and add direct fixture coverage for malformed inner objects.`
- NEXT_ESCALATION: `Promote validator-report nested sections with repeated failures into standalone schemas plus parser-level unit tests.`
- EXIT_CONDITION: `Nested governance payloads fail mechanically when child fields drift, truncate, or overstate proof.`

## ESCAPE
- ESCAPE_ID: `GOV-ESCAPE-TIMESTAMP-TYPING`
- STATUS: `SEEDED`
- TITLE: `Non-typed timestamp validation`
- TRIGGER_AUDIT: `.GOV/Audits/audits/AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT.md`
- FAILURE_SHAPE: `Timestamp-like fields were accepted as generic strings instead of being defended as typed RFC3339 UTC values with consistent authority semantics.`
- CANONICAL_CHECKS: `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`, `.GOV/roles_shared/checks/session-control-runtime-check.mjs`, `.GOV/roles_shared/checks/computed-policy-gate-check.mjs`
- CANONICAL_ASSETS: `timestamp_utc`, `last_event_at`, `last_heartbeat_at`, `heartbeat_due_at`, `stale_after`
- PROMOTION_RULE: `If timestamp drift reappears, promote the field family into a shared schema invariant with dedicated parser fixtures instead of per-check regex reuse.`
- NEXT_ESCALATION: `Extract repeated timestamp fields into one shared timestamp contract helper and require all new runtime ledgers to consume it.`
- EXIT_CONDITION: `Authority timestamps are validated as typed RFC3339 UTC fields wherever they control workflow routing, audit reconstruction, or staleness.`

## ESCAPE
- ESCAPE_ID: `GOV-ESCAPE-LEGACY-AUTHORITY-POISONING`
- STATUS: `ENFORCED`
- TITLE: `Legacy authority path poisoning live checks`
- TRIGGER_AUDIT: `.GOV/Audits/audits/AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT.md`
- FAILURE_SHAPE: `Repo-local legacy authority surfaces or compatibility branches remained capable of poisoning active startup, runtime, communication, or closure checks.`
- CANONICAL_CHECKS: `.GOV/roles_shared/checks/migration-path-truth-check.mjs`, `.GOV/roles_shared/checks/deprecation-sunset-check.mjs`, `.GOV/roles_shared/checks/wp-communications-check.mjs`
- CANONICAL_ASSETS: `.GOV/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md`, `.GOV/roles_shared/records/COMPATIBILITY_SHIM_LEDGER.md`, `../gov_runtime/roles_shared/*`
- PROMOTION_RULE: `If a legacy path poisons live checks again, add a shim-ledger entry, a deprecation-plan entry, and a dedicated migration-path invariant in the same patch.`
- NEXT_ESCALATION: `Delete the compatibility branch after the shim ledger and sunset trigger both show no remaining live dependency.`
- EXIT_CONDITION: `Legacy compatibility surfaces are explicit, bounded, and unable to silently act as live authority.`
