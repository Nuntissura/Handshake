# Integration Validator Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: INTEGRATION_VALIDATOR

## Use

Use this brief after `just validator-startup INTEGRATION_VALIDATOR`. It is operational memory for final-lane judgment.

## Action Cards

### RAM-INTEGRATION_VALIDATOR-CLOSEOUT-001

- ACTION: CLOSEOUT
- TRIGGER: before final verdict, merge, or sync-gov-to-main
- FAILURE_PATTERN: rebuilding packet/runtime/main compatibility truth manually or trusting prior role summaries
- DO: run startup, `validator-next`, and `just integration-validator-context-brief WP-{ID}` before broad repo search
- DO_NOT: use `handshake_main/.GOV` as live governance authority when `HANDSHAKE_GOV_ROOT` points to the kernel
- VERIFY: context brief prints packet path, prepare worktree, main compatibility, and closeout blockers
- SOURCE: INTEGRATION_VALIDATOR_PROTOCOL, GOV-CHANGE-20260429-03

### RAM-INTEGRATION_VALIDATOR-VERDICT-001

- ACTION: VERDICT
- TRIGGER: writing PASS, FAIL, merge, or status-sync truth
- FAILURE_PATTERN: treating WP Validator evidence or green tests as final authority
- DO: perform independent whole-WP review with spec clause map and file:line evidence
- DO_NOT: pass with debt for hard invariants, security, traceability, or spec alignment
- VERIFY: verdict report includes direct evidence and any remediation instructions are packet-visible
- SOURCE: INTEGRATION_VALIDATOR_PROTOCOL
