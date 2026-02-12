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

# Work Packet Stub: WP-1-LLM-Provider-Registry-v1

## STUB_METADATA
- WP_ID: WP-1-LLM-Provider-Registry-v1
- BASE_WP_ID: WP-1-LLM-Provider-Registry
- CREATED_AT: 2026-02-11T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER (non-authoritative): Phase 1 model runtime integration + portability
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - 4.2 Model Runtime (provider portability; OpenAI-compatible APIs; local-first stance)
  - 4.3.7 Work Profile System (role-based model assignment)
  - 4.3.3.4 ModelSwapRequest (swap + resume semantics)
  - 11.1.7 Cloud Escalation Consent Artifacts (human-gated external calls)
  - Cloud leakage rules (ModelTier::Cloud guarded by leakage validators)

## PROBLEM_STATEMENT (DRAFT)
- Product intent: Handshake must be able to use both local models and cloud models.
- Current state: local provider support exists (Ollama), but "cloud" is mostly a policy/tier concept
  and not an end-to-end provider integration surface.
- Without a provider registry and cloud adapters:
  - Work Profiles cannot be fully realized (assignments may reference unavailable providers).
  - Escalation chains that include cloud cannot execute.
  - Model "spin up" is ad-hoc or impossible, and onboarding new providers is non-deterministic.

## INTENT (DRAFT)
- What: Implement a deterministic, auditable LLM provider registry and at least one cloud-capable
  provider adapter (OpenAI-compatible HTTP API), integrated with Work Profiles, ModelSwap, and
  cloud consent enforcement.
- Why: Product must be project/task agnostic and portable across environments, while preserving
  strict safety (cloud leakage guard + explicit consent) and strong observability.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Provider registry:
    - Persisted provider configs (local + cloud) with stable IDs and redacted secrets.
    - Provider health/readiness checks and "available models" listing (best-effort).
  - Provider adapters:
    - Define a generic "OpenAI-compatible" adapter (covers OpenAI, vLLM, TGI, etc).
    - Ensure all adapters implement the existing `LlmClient` trait contract.
  - Tier + safety integration:
    - Cloud providers MUST be ModelTier=Cloud (leakage rules apply).
    - Enforce Cloud Escalation Consent artifacts for any cloud invocation when policy requires it.
  - Work Profile integration:
    - Work Profiles reference model assignments that include provider_id + model_id (or equivalent).
  - Telemetry:
    - Emit provider/adapter lifecycle events (start, fail, retry, timeout) without leaking secrets.
- OUT_OF_SCOPE:
  - Agentic tool-use frameworks for cloud models (this WP targets non-agentic completions + swaps).
  - Multi-tenant enterprise key management (Phase 2+).

## ACCEPTANCE_CRITERIA (DRAFT)
- A workspace can configure:
  - At least one local provider (Ollama), and
  - At least one cloud provider endpoint via OpenAI-compatible adapter.
- Work Profile model assignments can select provider+model deterministically.
- Cloud tier invocations are blocked when:
  - Consent is missing (when required by policy), or
  - Leakage validators flag non-exportable/high-sensitivity content.
- ModelSwapRequest can target a different provider+model and resume after fresh context compile.

## DESIGN_SKETCH (DRAFT)
- ProviderRegistry data model (minimum):
  - `provider_id` (stable, human-readable), `provider_kind` (e.g. ollama, openai_compat),
    `tier` (Local|Cloud), `base_url`, optional `default_model`, `created_at`, `updated_at`.
  - Secrets (API keys/tokens) must be stored separately from normal config and must not appear in:
    logs, debug bundles, governance pack exports, or flight recorder payloads.
- OpenAI-compatible adapter requirements (minimum):
  - Deterministic request assembly (temperature/top_p/max_tokens defaults documented).
  - Policy-driven timeout + retry rules with telemetry events (no payload leakage).
  - Best-effort `list_models()` support (allowed to be partial or cached).
- Work Profile integration (minimum):
  - Work Profile model assignment selects `(provider_id, model_id)` deterministically.
  - ModelSwapRequest includes provider change with a "fresh context compile" boundary.
- Guardrails:
  - Any network call to Cloud tier MUST be explicitly tiered + consent gated (when policy requires).
  - Any Cloud-tier invocation MUST be leakage-validated prior to send.

## VALIDATOR_RUBRIC_HOOKS (DRAFT)
- Safety: cloud calls are consent-gated and leakage-guarded; secrets never appear in logs/artifacts.
- Determinism: provider selection is stable given config; retries/timeouts are policy-defined and observable.
- Auditability: lifecycle telemetry exists for provider failures without payload leakage.
- Portability: adapter is OpenAI-compatible (covers multiple backends) and avoids vendor lock-in.

## VALIDATION_PLAN (DRAFT)
- Unit tests:
  - Provider config parsing/redaction (no secrets in logs or exported bundles).
  - OpenAI-compatible adapter request/response contract (mock server).
  - Enforcement tests: cloud call denied without ConsentReceipt when policy requires it.
- Integration:
  - Minimal end-to-end job using local provider, then swap to cloud provider (if enabled) and
    ensure events are recorded and no raw payloads leak.

## RED_TEAM / ABUSE_CASES (DRAFT)
- RT-CLOUD-001: Attempt cloud invocation with missing ConsentReceipt (must hard block).
- RT-CLOUD-002: Attempt to exfiltrate secrets via model prompt (leakage guard must block in Cloud tier).
- RT-SECRETS-001: Provider API key appears in logs, artifacts, debug bundles, or governance pack exports.
- RT-SSR-001: Malicious provider base_url points to attacker-controlled endpoint; ensure consent + leakage
  gates apply and all network calls are explicitly tiered/audited.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: Work Profiles baseline (or implement minimal subset inside this WP).
- Depends on: Cloud Escalation Consent artifact flow (may be stubbed/incomplete; integrate as needed).

## RISKS / UNKNOWNs (DRAFT)
- Risk: scope creep into full "provider marketplace". Mitigation: implement only registry + one
  cloud adapter + required governance gates.
- Risk: deterministic behavior vs provider variability. Mitigation: define strict timeout/retry
  policy and record requested vs effective behavior in telemetry.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm anchor sections exist in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-LLM-Provider-Registry-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-LLM-Provider-Registry-v1` (in `.GOV/task_packets/`).
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
