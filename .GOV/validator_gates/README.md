# Validator Gate State (Per-WP)

This directory contains the mechanical Validator gate state files written by:
- `just validator-gate-present {WP_ID} {PASS|FAIL}`
- `just validator-gate-acknowledge {WP_ID}`
- `just validator-gate-append {WP_ID}`
- `just validator-gate-commit {WP_ID}`
- `just validator-gate-status {WP_ID}`
- `just validator-gate-reset {WP_ID} --confirm`

State is stored per WP as:
- `.GOV/validator_gates/{WP_ID}.json`

Why: avoids cross-WP merge conflicts that occur when multiple validations append to a single global JSON ledger.

Legacy:
- `.GOV/roles/validator/VALIDATOR_GATES.json` is treated as a legacy archive for older sessions; new sessions should not write to it.

