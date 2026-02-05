# EVIDENCE_LEDGER

This document defines how evidence is recorded so Validator audits do not rely on narrative summaries.

---

## Scope and boundary

- This ledger is GOVERNANCE-ONLY.
- Handshake product runtime (code under `/src/`, `/app/`, `/tests/`) MUST NOT read from or write to `/.GOV/`.

See: `Handshake Codex v1.4.md` ([CX-211], [CX-212]) and `/.GOV/roles_shared/BOUNDARY_RULES.md`.

---

## Canonical evidence location (HARD)

The canonical evidence for a WP lives INSIDE the WP task packet (append-only):
- Path: `/.GOV/task_packets/{WP_ID}.md`
  - `## EVIDENCE` (commands, proof lines, hashes, artifacts)
  - `## VALIDATION_REPORTS` (Validator report + verdict)

Codex authority: [CX-657] CANONICAL_EVIDENCE_IN_PACKET (HARD).

---

## Supporting ledgers (machine-owned; not canonical)

1) Validator gate state (per WP)
- Path: `/.GOV/validator_gates/{WP_ID}.json`
- Purpose: deterministic per-WP validator gate state (machine-owned, merge-safe).

2) Role Mailbox export (optional helper)
- Path: `/.GOV/ROLE_MAILBOX/index.json` and `/.GOV/ROLE_MAILBOX/export_manifest.json`
- Purpose: leak-safe metadata exports of role decisions, approvals/waivers, and tooling results (no raw bodies).
- Verified by: `just role-mailbox-export-check`

The Role Mailbox stores metadata/hashes and pointers. Raw outputs stay in the task packet evidence (and/or pasted to chat when needed).

---

## Agentic minimum (required for agentic runs)

For every command that can block/proceed (examples: `just pre-work`, `just post-work`, `just validator-dal-audit`, `cargo test`):

A) In the task packet under `## EVIDENCE`, append:
- COMMAND: (exact)
- EXIT_CODE: (int)
- WORKTREE_DIR / BRANCH: (exact)
- GIT_SHA_BEFORE / GIT_SHA_AFTER: (exact)
- OUTPUT_SHA256: sha256 of the raw stdout/stderr bundle
- PROOF_LINES: paste 1-10 critical lines (avoid chat truncation; keep it auditable)

B) Optionally also write a matching Role Mailbox entry containing the same OUTPUT_SHA256 + metadata, but NOT the raw output body.

---

## Non-agentic (human-run)

Same as above, but the Role Mailbox is optional. The task packet remains canonical evidence.
