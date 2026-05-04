# AUDIT-20260504-WP1-DIRECT-REVIEW-INTENT-RECONCILIATION

- AUDIT_ID: AUDIT-20260504-WP1-DIRECT-REVIEW-INTENT-RECONCILIATION
- DATE: 2026-05-04
- LANE: Repo Governance
- WP_ID: WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- RESULT: APPLIED

## Finding

During the live orchestrator-managed run, `CODER_INTENT` was blocked by two governance mechanics:

- the direct-review wrapper required `ack_for` but did not make it ergonomic for the coder intent path
- the canonical coder intent closed the canonical validator kickoff but left the earlier malformed duplicate kickoff open, which kept runtime routing stuck on CODER

## Change

- `wp-review-exchange.mjs` now defaults `ack_for` to `correlation_id` for CODER-owned `CODER_INTENT` receipts when the wrapper omits it.
- `wp-receipt-append.mjs` now resolves stale duplicate `VALIDATOR_KICKOFF` open items when a CODER-owned `CODER_INTENT` targets the same validator/coder sessions and microtask scope.
- Focused tests cover both behaviors.

## Verification

- `node --check .GOV\roles_shared\scripts\wp\wp-receipt-append.mjs`
- `node --check .GOV\roles_shared\scripts\wp\wp-review-exchange.mjs`
- `node --test --test-name-pattern "coder intent resolves superseded|resolveReviewAckFor|parseReviewExchange|deriveFallbackReviewMicrotaskContract" .GOV\roles_shared\tests\wp-receipt-append.test.mjs .GOV\roles_shared\tests\wp-review-exchange.test.mjs`

## Runtime Outcome

The active WP lane advanced from the stale duplicate kickoff state to `WP_VALIDATOR_INTENT_CHECKPOINT`, with the WP Validator as the next expected actor.
