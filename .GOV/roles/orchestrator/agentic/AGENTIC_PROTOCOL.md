# AGENTIC_PROTOCOL (Orchestrator)

This is an **add-on** protocol for orchestrator-led, multi-agent ("agentic") execution.

It does not replace `/.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`; it adds constraints to prevent drift when multiple agents are running.

---

## 0) Packet metadata (HARD)

If a WP is run in agentic mode:
- The task packet `## METADATA` MUST set:
  - `AGENTIC_MODE: YES`
  - `ORCHESTRATOR_MODEL: ...`
  - `ORCHESTRATION_STARTED_AT_UTC: ...` (RFC3339 UTC)

Rationale: makes multi-agent provenance auditable even if chat context is truncated.

---

## 1) Single decision-maker rule (HARD)

- The Orchestrator is the sole decision-maker.
- Sub-agents (Coder/Validator/Advisory) may propose, but must not decide scope, waivers, or "Done" status.

---

## 2) Artifact-first continuity (HARD)

Every sub-agent instruction MUST include the canonical artifact set:
- `Handshake Codex v1.4.md`
- `/.GOV/roles/<role>/*_PROTOCOL.md` (role protocol)
- `/.GOV/roles_shared/SPEC_CURRENT.md` (+ the referenced Master Spec file)
- Active task packet in `/.GOV/task_packets/{WP_ID}.md`
- Refinement in `/.GOV/refinements/{WP_ID}.md` (if applicable)
- `/.GOV/roles_shared/BOUNDARY_RULES.md`

Do not rely on "what the agent remembers". Assume each agent starts with near-zero context.

---

## 3) Gate outputs are not optional (HARD)

- No agent may claim PASS/FAIL without the literal tool output available to the Validator.
- If outputs are too large for chat, record hashes + metadata per `/.GOV/roles_shared/EVIDENCE_LEDGER.md` and keep the raw output retrievable on request.

---

## 4) Evidence ledger (HARD)

- For agentic runs, maintain the evidence ledger per `/.GOV/roles_shared/EVIDENCE_LEDGER.md`.
- Ensure RoleMailbox export remains leak-safe and passes `just role-mailbox-export-check`.

---

## 5) Delegation template (copy/paste)

When delegating to an agent, use this exact structure:

1) ROLE + WP:
- ROLE: Coder|Validator|Advisory:<name>
- WP_ID: <WP-...>
- Worktree + branch: <dir> + <branch>

2) Canonical artifacts (paths):
- Codex: `Handshake Codex v1.4.md`
- Role protocol: `/.GOV/roles/<role>/*_PROTOCOL.md`
- Spec pointer: `/.GOV/roles_shared/SPEC_CURRENT.md`
- Task packet: `/.GOV/task_packets/<WP_ID>.md`
- Refinement: `/.GOV/refinements/<WP_ID>.md` (if any)

3) Scope (paths only):
- IN_SCOPE_PATHS: ...
- OUT_OF_SCOPE: ...

4) Required commands (exact):
- RUN_COMMANDS: ...

5) Evidence requirements:
- Paste literal outputs for all gate commands OR record ledger entries with sha256.

6) Stop condition:
- "If anything is missing/ambiguous: STOP and ask."
