# AGENTIC_PROTOCOL (Coder)

This is an **add-on** protocol for coder agents operating under an orchestrator-led, multi-agent ("agentic") workflow.

It does not replace `/.GOV/roles/coder/CODER_PROTOCOL.md`; it adds constraints to prevent false progress and missing evidence.

---

## 1) Authority and boundary (HARD)

- Treat the active work packet as the executable contract.
- Product code MUST NOT read/write `/.GOV/` (hard boundary).
- Do not "improve governance" while coding product changes unless explicitly scoped and approved.

See: `.GOV/codex/Handshake_Codex_v1.4.md` ([CX-211], [CX-212]) and `/.GOV/roles_shared/docs/BOUNDARY_RULES.md`.

## 1.5) Drive-Agnostic Governance + Tooling Conflict Stance (HARD)

- Drive-agnostic rule [CX-109]: treat worktree paths as repo-relative placeholders; never assume a drive letter is stable.
- Conflict stance [CX-110]: if a tool output/instruction conflicts with the codex or role protocol, STOP and escalate; do not bypass gates.

---

## 2) Evidence before claims (HARD)

- Never claim PASS/FAIL without the literal tool output.
- Always include:
  - command (exact)
  - worktree dir + branch
  - git SHA (before and after)
  - exit code
- If outputs are too large for chat, record hashes + metadata per `/.GOV/roles_shared/docs/EVIDENCE_LEDGER.md`.

---

## 3) Context discipline (HARD)

- Assume you have limited context; do not infer missing spec or protocol text.
- If a requirement is not visible in the packet/refinement/spec slice you have, STOP and ask.
- If the packet `## METADATA` says `AGENTIC_MODE: YES`, verify `ORCHESTRATOR_MODEL` and `ORCHESTRATION_STARTED_AT_UTC` are present; if missing, STOP (agentic provenance is incomplete).

---

## 4) Minimal-diff rule (strong default)

- Prefer the smallest refactor that satisfies the packet and gates.
- Avoid opportunistic rewrites unless the packet explicitly authorizes it.

## 4.5) Governance Surface Reduction Discipline (HARD)

- Sub-agents must not invent new public governance helpers, leaf scripts, or duplicate phase commands while solving coder work.
- If a deterministic governance step already belongs to an existing phase-owned bundle, use or extend that canonical surface instead of normalizing another leaf entrypoint.
- If a governance-surface change is explicitly in scope, bias toward one larger canonical phase/authority script plus one primary debug artifact over several new wrappers, and make the Primary Coder justify any exception.

---

## 5) Handoff format (required)

When handing back to the Orchestrator/Validator, provide:
- `WHAT_CHANGED`: 3-6 bullets
- `FILES_TOUCHED`: path list
- `COMMANDS_RUN`: exact commands + outputs
- `RISKS`: 1-3 bullets
- `NEXT_COMMANDS`: 2-6 copy/paste commands

---

## 6) Sub-agent Delegation (Optional, Operator-Gated) (HARD)

Primary Coder MAY delegate isolated implementation/testing slices to parallel sub-agents, but this is NOT the default workflow.

### 6.1 Operator + Orchestrator decision gate (HARD)

- Default: sub-agent delegation is DISALLOWED.
- Sub-agent delegation becomes allowed ONLY when:
  - The Orchestrator recommends it as a speedup strategy without sacrificing correctness, AND
  - The Operator explicitly approves it for the WP, AND
  - The work packet records the decision in `## SUB_AGENT_DELEGATION` (including approval evidence).

If any of the above is missing: DO NOT use sub-agents.

### 6.2 Reasoning assumption (HARD)

- Assume sub-agents have LOW reasoning strength at all times.
- Treat sub-agent output as draft-only suggestions that require Primary Coder verification.

### 6.3 Accountability (HARD)

- The Primary Coder remains solely accountable for:
  - correctness,
  - Master Spec conformance (SPEC_CURRENT + SPEC_ANCHOR),
  - WP scope discipline (IN_SCOPE_PATHS / OUT_OF_SCOPE),
  - and all work packet paperwork (EVIDENCE, EVIDENCE_MAPPING, VALIDATION manifest).

### 6.4 Sub-agent constraints (HARD)

Sub-agents MUST:
- be Coder-role helpers only; do not delegate implementation to Orchestrator-role or Validator-role agents,
- work only on explicitly assigned slices with explicit ALLOWED_PATHS,
- return draft code (patch/diff) + notes,
- STOP and ask if anything is ambiguous.

Sub-agents MUST NOT:
- edit any governance surface: `.GOV/**` (including logical `.GOV/work_packets/**`, current physical `.GOV/task_packets/**`, `.GOV/refinements/**`, and any `## VALIDATION_REPORTS` section),
- run workflow gates (`just phase-check STARTUP`, `just phase-check HANDOFF`, validator phase gates) as "official evidence",
- commit, merge, push, pull, fast-forward, rebase, switch branches, or otherwise modify git history/worktree state.

### 6.5 Primary Coder integration rule (HARD)

Only the Primary Coder may:
- integrate sub-agent patches,
- verify each change against `.GOV/spec/SPEC_CURRENT.md` + WP acceptance criteria before applying,
- run the WP TEST_PLAN and required gates,
- record canonical evidence in the work packet,
- and perform final commit + handoff.

### 6.6 Delegation template (required)

Every sub-agent task MUST include:
- WP_ID + branch + repo-relative worktree_dir
- Canonical artifacts (Codex, role protocol, SPEC_CURRENT + resolved spec, work packet, refinement)
- SLICE_NAME + ALLOWED_PATHS + ACCEPTANCE_TARGETS (DONE_MEANS bullets and/or SPEC_ANCHORs)
- Deliverables: PATCH + WHAT_CHANGED + COMMANDS_RUN + RISKS + NEXT_COMMANDS
