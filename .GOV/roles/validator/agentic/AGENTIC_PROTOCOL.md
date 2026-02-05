# AGENTIC_PROTOCOL (Validator)

This is an **add-on** protocol for validators operating in orchestrator-led, multi-agent ("agentic") workflows.

It does not replace `/.GOV/roles/validator/VALIDATOR_PROTOCOL.md`; it adds failure-mode defenses specific to agentic relays.

---

## 1) Default distrust of summaries (HARD)

- Treat orchestrator/coder summaries as untrusted until backed by evidence.
- Base verdicts on:
  - repo state (git SHA + diffs)
  - gate outputs
  - spec-to-code mapping

---

## 2) Gate evidence requirement (HARD)

- If a gate is relevant, lack of literal output is a FAIL unless waived.
- If the orchestrator says "gate passed", but cannot provide the output (or a verifiable hash + retrieval path), treat it as NOT RUN.

Evidence ledger reference: `/.GOV/roles_shared/EVIDENCE_LEDGER.md`.

---

## 3) Worktree/branch misdirection defense (HARD)

- Always run the worktree gate (`pwd`, `git rev-parse`, `git status -sb`, `git worktree list`) and paste outputs.
- Re-run gates against the WP worktree recorded in `/.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` (PREPARE record), not against your role worktree copy.

---

## 4) "Range vs worktree" trap (HARD)

- If post-work is run with `--range`, it validates COMMITTED blobs only.
- Uncommitted worktree diffs are invisible to range checks.
- Therefore: do not accept `post-work --range base..HEAD` as evidence for uncommitted changes.

---

## 5) Role Mailbox use (recommended)

When the run is agentic:
- Require Role Mailbox export metadata to be maintained and to pass `just role-mailbox-export-check`.
- This is not a substitute for spec-to-code mapping, but it prevents "decision drift" in multi-agent relays.
