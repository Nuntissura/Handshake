# SPEC_DEBT_REGISTRY

Rules:
- Every `SPECDEBT-*` referenced by a work packet must appear exactly once in this registry.
- Use `STATUS: OPEN` while the debt remains unresolved against the current packet/spec truth.
- Use `BLOCKING: YES` only when the debt blocks a full spec PASS for the packet.
- Keep rows append-only for history; close debt by changing `STATUS: CLOSED` in a governed follow-up edit, not by deleting the row.
- Preferred workflow: use `just spec-debt-open`, `just spec-debt-sync`, and `just spec-debt-close` instead of manual freehand edits.

## DEBT_ROWS
- NONE
