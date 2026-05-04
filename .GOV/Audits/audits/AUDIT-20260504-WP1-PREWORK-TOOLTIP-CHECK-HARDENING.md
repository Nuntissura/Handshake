# AUDIT-20260504-WP1-PREWORK-TOOLTIP-CHECK-HARDENING

- AUDIT_ID: AUDIT-20260504-WP1-PREWORK-TOOLTIP-CHECK-HARDENING
- DATE: 2026-05-04
- LANE: Repo Governance
- WP_ID: WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- RESULT: APPLIED

## Finding

`pre-work-check.mjs` rejected a valid packet whose `UI_CONTROLS` entries used the task-template form `Tooltip: text`.

The check used a word-boundary match after the colon, which does not match a colon followed by whitespace. The failure blocked `phase-check STARTUP` even though the packet was hydrated with concrete tooltip text.

## Change

- `pre-work-check.mjs` now accepts `Tooltip:` followed by non-placeholder text.
- Existing `<fill>` placeholders remain rejected.

## Verification

- `node --check .GOV\roles\coder\checks\pre-work-check.mjs`
- `just phase-check STARTUP WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1 CODER coder:wp-1-software-delivery-validator-gate-closeout-posture-v1`
- `just gov-check`

## Runtime Outcome

The active WP startup gate can evaluate the packet's concrete UI controls without a false tooltip-blocker.
