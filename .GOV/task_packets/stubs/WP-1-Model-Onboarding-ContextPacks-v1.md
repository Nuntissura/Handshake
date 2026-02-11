# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in
  `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Model-Onboarding-ContextPacks-v1

## STUB_METADATA
- WP_ID: WP-1-Model-Onboarding-ContextPacks-v1
- BASE_WP_ID: WP-1-Model-Onboarding-ContextPacks
- CREATED_AT: 2026-02-11T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER (non-authoritative): prompt -> spec -> macro -> micro pipeline + governed multi-model execution
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - 2.6.8 Prompt-to-Spec Governance Pipeline (role contracts + routing)
  - 2.6.6.8 Micro-Task Executor Profile (iterative loop + escalation)
  - 4.3.7 Work Profiles (role-based model assignment)
  - 4.3.9 Multi-Model Orchestration & Lifecycle Telemetry (role execution identity)
  - Governance Pack / Template Volume (project-agnostic codex/protocol material)

## PROBLEM_STATEMENT (DRAFT)
- Product requirement: Handshake must be able to spin up and use multiple models (local or cloud),
  including non-agentic models, and onboard them correctly to:
  - the right role identity (orchestrator/coder/validator/etc),
  - the right Codex/governance constraints for the current project/workspace,
  - the right task context (macro WP, micro-task, required files, and verification expectations).
- Current risk pattern:
  - Role instructions are inconsistently assembled or hardcoded.
  - Swaps/escalations can lose constraints ("fresh model, fresh rules" problem).
  - Different providers/models receive different effective instructions.

## INTENT (DRAFT)
- What: Define and implement deterministic "Context Packs" used to onboard any model instance to a
  role + task, including on swap/escalation, with strict provenance, hashing, and auditability.
- Why: Without deterministic onboarding artifacts, multi-model execution becomes non-reproducible
  and governance enforcement drifts per provider/model.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Context Pack schema:
    - Stable, versioned schema for "RoleContextPack" and "WorkUnitContextPack".
    - Includes: role_id, codex/protocol references, governance mode, allowed tools, lock-set,
      task identifiers (WP/MT IDs), and required verification contract.
  - Deterministic assembly:
    - Canonical ordering rules, stable hashing, and redacted exports (no secrets).
    - Packs emitted as artifacts so they can be replayed for audit/debug.
  - Runtime integration:
    - On job start: build a Context Pack and bind it to the job/workflow run.
    - On ModelSwap/escalation: rebuild pack (or explicitly reuse if byte-identical) and record
      "requested vs effective" pack hashes.
  - Provider neutrality:
    - Context Pack must be usable for local and cloud providers; no provider-specific prompt magic.
  - Operator UX (minimum viable):
    - Show a compact summary: role, model/provider, pack_hash, lock paths, and whether cloud tier is used.
- OUT_OF_SCOPE:
  - Full UI design polish.
  - Training/distillation flows (Phase 2+).

## ACCEPTANCE_CRITERIA (DRAFT)
- Every model invocation in a governed workflow can be traced to:
  - a RoleExecutionIdentity and
  - a Context Pack artifact (hash + storage ref).
- ModelSwap/escalation cannot proceed without a valid Context Pack for the target role.
- Context Pack assembly is deterministic: same inputs -> same pack hash.
- Context Packs do not embed repo-absolute paths or host-specific drive letters.

## VALIDATOR_RUBRIC_HOOKS (DRAFT)
- Determinism: stable ordering + hashing; swap/escalation produces traceable pack hashes (requested vs effective).
- Governance fidelity: role and codex/protocol constraints do not change across providers or swaps.
- Safety: no secrets in packs; cloud tier uses projection/consent rules (no raw leakage).
- Recoverability: packs allow replay/debug of what the model actually saw.

## VALIDATION_PLAN (DRAFT)
- Unit tests:
  - Deterministic hashing and stable ordering.
  - Redaction: secrets never appear in pack artifacts.
  - Boundary: pack references do not require repo `.GOV/**` or `docs/**` runtime reads.
- Integration:
  - Run a micro-task iteration, force a swap/escalation, and verify:
    - a new pack is emitted (or reused deterministically),
    - telemetry captures both hashes,
    - behavior remains within the same governance constraints.

## RED_TEAM / ABUSE_CASES (DRAFT)
- RT-ROLE-001: Model is invoked with the wrong role_id or wrong codex/protocol version (must be detected).
- RT-SWAP-001: Swap occurs and the new model loses restrictions (must hard fail or rebuild pack).
- RT-INJECT-001: Prompt injection attempts to disable gates or request forbidden files/tools (pack must not allow).
- RT-LEAK-001: Context Pack includes secrets (keys, tokens) or sensitive content that could leak to cloud.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: provider registry / adapters (WP-1-LLM-Provider-Registry-v1) for multi-provider testing.
- Depends on: Work Profiles or equivalent role assignment surface.

## RISKS / UNKNOWNs (DRAFT)
- Risk: pack content becomes too large for small models. Mitigation: pack must separate "strict rules"
  (small, always included) from "context payload" (retrieval-assembled per request) with explicit budgets.
- Risk: spec ambiguity about where codex/protocol material lives for arbitrary projects. Mitigation:
  coordinate with spec enrichment WP to define canonical sources and avoid repo coupling.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm anchor sections exist in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Model-Onboarding-ContextPacks-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Model-Onboarding-ContextPacks-v1` (in `.GOV/task_packets/`).
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
