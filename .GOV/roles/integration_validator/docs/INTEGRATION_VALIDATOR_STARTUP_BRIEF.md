# Integration Validator Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: INTEGRATION_VALIDATOR

## Use

Use this brief after `just validator-startup INTEGRATION_VALIDATOR`. It is operational memory for final-lane judgment.

## Action Cards

### RAM-INTEGRATION_VALIDATOR-MECHANICAL_INTERVENTION-001

- ACTION: MECHANICAL_INTERVENTION
- TRIGGER: before final verdict, merge containment, status sync, declaring a stall, or treating handoff/documentation/protocol drift as blocked
- FAILURE_PATTERN: re-deriving final-lane truth manually or running terminal closeout before resolving the open final handoff correlation
- DO: classify 3-5 plausible causes including runtime route drift, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, and scope/worktree drift; then use the cheapest deterministic read, phase check, merge-containment proof, or typed helper before mutating verdict/main truth
- DO_NOT: manually relay ordinary final review content when `phase-check VERDICT`, `wp-review-response`, contained-main closeout, or integration-validator context helpers own the state transition
- VERIFY: final action cites the cause class, helper output, original handoff correlation, packet target head, and current main containment evidence
- SOURCE: CX-218K, INTEGRATION_VALIDATOR_PROTOCOL, .GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md

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
