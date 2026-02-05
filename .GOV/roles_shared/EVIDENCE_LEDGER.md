# EVIDENCE_LEDGER

This document defines the governance "evidence ledger" concept used to make agentic work auditable without trusting narrative summaries.

---

## Scope and boundary

- This ledger is GOVERNANCE-ONLY.
- Handshake product runtime (code under `/src/`, `/app/`, `/tests/`) MUST NOT read from or write to `/.GOV/`.

See: `Handshake Codex v1.4.md` ([CX-211], [CX-212]) and `/.GOV/roles_shared/BOUNDARY_RULES.md`.

---

## Canonical evidence artifacts

1) Gate state (per WP)
- Path: `/.GOV/validator_gates/{WP_ID}.json`
- Purpose: deterministic per-WP validator gate state (machine-owned, merge-safe).

2) Role Mailbox export (cross-role)
- Path: `/.GOV/ROLE_MAILBOX/index.json` and `/.GOV/ROLE_MAILBOX/export_manifest.json`
- Purpose: leak-safe metadata exports of role decisions, approvals/waivers, and tooling results (no raw bodies).
- Verified by: `just role-mailbox-export-check`

---

## Agentic minimum requirement (HARD for agentic runs)

For every gate/tooling command that can block/proceed (examples: `just pre-work`, `just post-work`, `just validator-dal-audit`, `cargo test`):

- Record an evidence entry in the Role Mailbox (as a `tooling_result` or `validation_finding`) that includes:
  - `timestamp` (RFC3339 UTC)
  - `role_id` (orchestrator|coder|validator)
  - `wp_id`
  - `worktree_dir` and `branch`
  - `command` (exact)
  - `exit_code`
  - `git_sha_before` and `git_sha_after`
  - `output_sha256` (sha256 of the raw stdout/stderr bundle)
  - `summary` (single line)

Raw outputs MUST remain available to the Validator on request (paste into chat or attach out-of-band), but MUST NOT be stored as raw message bodies in the Role Mailbox.

---

## Non-agentic (human-run)

Evidence can be recorded directly in the task packet under `## EVIDENCE` (verbatim outputs), and gate state should still be maintained via `/.GOV/validator_gates/{WP_ID}.json`.
