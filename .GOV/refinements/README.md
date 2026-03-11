# Refinements

This directory holds per-WP Technical Refinement artifacts created BEFORE task packet creation.

- Template: `.GOV/templates/REFINEMENT_TEMPLATE.md`
- Expected path: `.GOV/refinements/{WP_ID}.md`

These files are validated by workflow gates (context-token-in-window) and are required before:
- `just record-signature {WP_ID} {signature} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} {Coder-A|Coder-B}`
- `just create-task-packet {WP_ID}`

