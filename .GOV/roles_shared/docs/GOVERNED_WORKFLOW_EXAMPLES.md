# GOVERNED_WORKFLOW_EXAMPLES

**Status:** Draft  
**Intent:** concrete examples for the current orchestrator-managed governance workflow  
**Scope:** examples only; the authoritative law remains the active protocols, checks, and packet truth

## Purpose

These examples show the intended shape of the governed workflow using the current command surface.

They are not substitutes for:
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
- `.GOV/roles/coder/CODER_PROTOCOL.md`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`

## Example 1 - Coder -> WP Validator direct-review cycle

Use this when a normal orchestrator-managed WP is active and the coder needs to hand off to the WP validator without orchestrator relay.

### Orchestrator

```bash
just orchestrator-startup
just orchestrator-prepare-and-packet WP-{ID}
just launch-coder-session WP-{ID}
just launch-wp-validator-session WP-{ID}
```

Normal launch note:
- supported launch paths auto-issue the first governed `START_SESSION`, so ordinary orchestrator-managed startup should not require a second manual `start-*` step

### WP Validator opens the review lane

```bash
just validator-startup
just validator-next WP-{ID}
just wp-validator-kickoff WP-{ID} <wp_validator_session> <coder_session> "Review scope and tripwires for WP-{ID}"
just wp-communication-health-check WP-{ID} KICKOFF
```

### Coder acknowledges intent and implements

```bash
just coder-startup
just coder-next WP-{ID}
just check-notifications WP-{ID} CODER <coder_session>
just ack-notifications WP-{ID} CODER <coder_session>
just wp-coder-intent WP-{ID} <coder_session> <wp_validator_session> "Implementation plan and first proof target" <kickoff_correlation_id>
just pre-work WP-{ID}
just post-work WP-{ID}
just wp-coder-handoff WP-{ID} <coder_session> <wp_validator_session> "Committed handoff ready for review"
```

### WP Validator reviews the committed handoff

```bash
just check-notifications WP-{ID} WP_VALIDATOR <wp_validator_session>
just ack-notifications WP-{ID} WP_VALIDATOR <wp_validator_session>
just wp-communication-health-check WP-{ID} HANDOFF
just validator-handoff-check WP-{ID}
just wp-validator-review WP-{ID} <wp_validator_session> <coder_session> "Review findings or acceptance summary" <handoff_correlation_id>
```

### Why this example exists

- The direct-review pair is the governed proof surface.
- `THREAD.md` alone is not enough.
- The Orchestrator is not the message broker for ordinary coder <-> WP validator traffic.

## Example 2 - Coder -> WP Validator -> Integration Validator finalization

Use this when the WP is orchestrator-managed and final merge-ready authority belongs to `INTEGRATION_VALIDATOR`.

### Orchestrator launches the final validator lane

```bash
just launch-integration-validator-session WP-{ID}
```

### Integration Validator resumes and opens the final review pair

```bash
just validator-startup
just validator-next WP-{ID}
just wp-review-exchange REVIEW_REQUEST WP-{ID} INTEGRATION_VALIDATOR <intval_session> CODER <coder_session> "Final merge-readiness review request" "" "<spec_anchor>" "<packet_row_ref>"
```

### Coder responds directly to the Integration Validator

```bash
just check-notifications WP-{ID} CODER <coder_session>
just ack-notifications WP-{ID} CODER <coder_session>
just wp-review-response WP-{ID} CODER <coder_session> INTEGRATION_VALIDATOR <intval_session> "Final response with closure evidence" <review_request_correlation_id>
```

### Integration Validator clears final gates

```bash
just check-notifications WP-{ID} INTEGRATION_VALIDATOR <intval_session>
just ack-notifications WP-{ID} INTEGRATION_VALIDATOR <intval_session>
just wp-communication-health-check WP-{ID} VERDICT
just validator-gate-append WP-{ID} PASS
just validator-gate-commit WP-{ID}
just validator-gate-present WP-{ID} PASS
just validator-gate-acknowledge WP-{ID}
just gov-check
```

### Why this example exists

- `WP_VALIDATOR` is advisory for orchestrator-managed WPs.
- Final merge-ready authority is a separate lane.
- The final coder <-> integration-validator review pair must be visible in governed receipts before verdict clearance.

## Example 3 - Stale session recovery

Use this when the session registry, broker state, or worktree truth looks stale.

### Operator/Orchestrator read path

```bash
just session-registry-status WP-{ID}
just handshake-acp-broker-status
just orchestrator-next WP-{ID}
```

### Packet communication repair checks

```bash
just wp-communication-health-check WP-{ID} STATUS
just check-notifications WP-{ID} CODER <coder_session>
just check-notifications WP-{ID} WP_VALIDATOR <wp_validator_session>
```

### Repair rules

- If the packet is blocked, superseded, or terminal, do not resume the old governed session.
- If the assigned worktree is missing, do not steer the old session just because the registry still shows a thread id.
- If the broker is stale and no governed runs are active, stop the broker and let the next governed launch recreate it.
- Prefer governed close/recreate behavior over hand-editing runtime ledgers.

## Example 4 - Blocked legacy packet handling

Use this when a historical packet was once "validated" but is now explicitly blocked by current governance law.

### Read path

```bash
just validator-next WP-{ID}
just validator-policy-gate WP-{ID}
just session-registry-status WP-{ID}
```

### Expected handling

- If the packet reports `LEGACY_CLOSED_PACKET_REMEDIATION_REQUIRED`, treat it as historical failed closure.
- Do not reopen validator gates on that packet.
- Do not re-prepare or reassign it as if it were active execution.
- Open a new remediation packet/version instead.
- If the historical packet is still the comparison anchor for a recovery run, also record it under:
  - `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` -> `## Historical Failure + Live Smoketest Lineage`
  - `.GOV/roles_shared/records/TASK_BOARD.md` -> `## Historical Failed Closures Used As Live Smoketest Baselines`

## Example 5 - Parallel WP ownership at a glance

Use this as the minimal mental model for parallel governed work.

### Example shape

- `WP-A`
  - `CODER` in `wtc-WP-A`
  - `WP_VALIDATOR` in `wtv-WP-A`
- `WP-B`
  - `CODER` in `wtc-WP-B`
  - `WP_VALIDATOR` in `wtv-WP-B`
- shared repo lanes
  - `ORCHESTRATOR` in `wt-gov-kernel`
  - `INTEGRATION_VALIDATOR` in `handshake_main`

### Not allowed

- `WP-A` and `WP-B` sharing the same WP-specific worktree
- a separate ordinary validator-only WP worktree
- two concurrent active ACP runs for the same governed role/WP session
- using `WP_VALIDATOR` as final merge authority for orchestrator-managed closure
