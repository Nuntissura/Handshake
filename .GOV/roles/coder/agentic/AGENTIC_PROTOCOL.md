# AGENTIC_PROTOCOL (Coder)

This is an **add-on** protocol for coder agents operating under an orchestrator-led, multi-agent ("agentic") workflow.

It does not replace `/.GOV/roles/coder/CODER_PROTOCOL.md`; it adds constraints to prevent false progress and missing evidence.

---

## 1) Authority and boundary (HARD)

- Treat the active task packet as the executable contract.
- Product code MUST NOT read/write `/.GOV/` (hard boundary).
- Do not "improve governance" while coding product changes unless explicitly scoped and approved.

See: `Handshake Codex v1.4.md` ([CX-211], [CX-212]) and `/.GOV/roles_shared/BOUNDARY_RULES.md`.

---

## 2) Evidence before claims (HARD)

- Never claim PASS/FAIL without the literal tool output.
- Always include:
  - command (exact)
  - worktree dir + branch
  - git SHA (before and after)
  - exit code
- If outputs are too large for chat, record hashes + metadata per `/.GOV/roles_shared/EVIDENCE_LEDGER.md`.

---

## 3) Context discipline (HARD)

- Assume you have limited context; do not infer missing spec or protocol text.
- If a requirement is not visible in the packet/refinement/spec slice you have, STOP and ask.
- If the packet `## METADATA` says `AGENTIC_MODE: YES`, verify `ORCHESTRATOR_MODEL` and `ORCHESTRATION_STARTED_AT_UTC` are present; if missing, STOP (agentic provenance is incomplete).

---

## 4) Minimal-diff rule (strong default)

- Prefer the smallest refactor that satisfies the packet and gates.
- Avoid opportunistic rewrites unless the packet explicitly authorizes it.

---

## 5) Handoff format (required)

When handing back to the Orchestrator/Validator, provide:
- `WHAT_CHANGED`: 3-6 bullets
- `FILES_TOUCHED`: path list
- `COMMANDS_RUN`: exact commands + outputs
- `RISKS`: 1-3 bullets
- `NEXT_COMMANDS`: 2-6 copy/paste commands
