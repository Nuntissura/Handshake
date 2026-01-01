# Refinements

This directory holds per-WP Technical Refinement artifacts created BEFORE task packet creation.

- Template: `docs/REFINEMENT_TEMPLATE.md`
- Expected path: `docs/refinements/{WP_ID}.md`

These files are validated by workflow gates (context-token-in-window) and are required before:
- `just record-signature {WP_ID} {signature}`
- `just create-task-packet {WP_ID}`

