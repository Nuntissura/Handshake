# Task Packet: WP-1-LLM-Provider-Registry-v1

## METADATA
- TASK_ID: WP-1-LLM-Provider-Registry-v1
- WP_ID: WP-1-LLM-Provider-Registry-v1
- BASE_WP_ID: WP-1-LLM-Provider-Registry (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-12T03:19:16.812Z
- MERGE_BASE_SHA: fadbbeb81693b7aa82ecd7eb8eca78dfc28c0049
- REQUESTOR: ilja (Operator)
- AGENT_ID: CodexCLI-GPT-5.2
- ROLE: Orchestrator
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL: N/A
- ORCHESTRATION_STARTED_AT_UTC: N/A
- CODER_MODEL: CodexCLI-GPT-5.2
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja120220260340
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-LLM-Provider-Registry-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement a deterministic LLM Provider Registry and at least one cloud-capable adapter (OpenAI-compatible HTTP API) behind `LlmClient`, with tier tagging (Local|Cloud) and governance guards for Cloud tier usage (respect runtime policy + Consent artifacts when required). Wire provider selection into backend startup so the runtime can select provider+model deterministically.
- Why: The Master Spec requires provider portability (Single Client Invariant via `LlmClient`) and explicit governance/consent for cloud escalation; without a registry + cloud adapter, cloud tier cannot be implemented deterministically and model routing cannot be audited.
- IN_SCOPE_PATHS:
  - .GOV/task_packets/WP-1-LLM-Provider-Registry-v1.md
  - .GOV/task_packets/stubs/WP-1-LLM-Provider-Registry-v1.md
  - .GOV/refinements/WP-1-LLM-Provider-Registry-v1.md
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/llm/registry.rs
  - src/backend/handshake_core/src/llm/openai_compat.rs
  - src/backend/handshake_core/src/llm/guard.rs
  - src/backend/handshake_core/src/main.rs
  - src/backend/handshake_core/src/workflows.rs
- OUT_OF_SCOPE:
  - UI for provider management (settings screens, Operator Consoles)
  - Enterprise key management / multi-tenant secrets rotation
  - Provider-specific features beyond OpenAI-compatible completions (tools/function-calling, streaming) unless required to satisfy `LlmClient` contract
  - Any weakening of cloud escalation consent / leakage gates

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-LLM-Provider-Registry-v1

# Targeted backend tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml llm

# Full backend tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Mechanical scan gates:
just product-scan

just cargo-clean
just post-work WP-1-LLM-Provider-Registry-v1 --range fadbbeb81693b7aa82ecd7eb8eca78dfc28c0049..HEAD
```

### DONE_MEANS
- A provider registry exists that can deterministically resolve an effective `(provider_id, tier, base_url, model_id)` for runtime roles.
- An OpenAI-compatible adapter implements `LlmClient::completion(...)` and is tested against a local mock server (no real network dependency in tests).
- Cloud tier invocations respect runtime policy: when cloud escalation is disallowed by policy, cloud completion attempts are hard-blocked (covered by unit/integration test).
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` + `just product-scan` + `just post-work WP-1-LLM-Provider-Registry-v1 --range fadbbeb81693b7aa82ecd7eb8eca78dfc28c0049..HEAD` all PASS.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.125.md (recorded_at: 2026-02-12T03:19:16.812Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 4.2.3 (LLM Client Adapter, Normative) + 4.3.7 (Work Profile System, Normative) + 11.1.7 (Cloud Escalation Consent Artifacts, Normative)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- BASE_WP_ID: WP-1-LLM-Provider-Registry
- v1 (THIS PACKET; activated from stub):
  - Stub source: .GOV/task_packets/stubs/WP-1-LLM-Provider-Registry-v1.md
  - Prior official packets: NONE
  - Purpose of v1: first activation into an executable packet with signed refinement + deterministic gates.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - .GOV/refinements/WP-1-LLM-Provider-Registry-v1.md
  - .GOV/task_packets/stubs/WP-1-LLM-Provider-Registry-v1.md
  - Handshake_Master_Spec_v02.125.md (anchors: 4.2.3, 4.3.7, 11.1.7)
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/main.rs
  - src/backend/handshake_core/src/workflows.rs
- SEARCH_TERMS:
  - "init_llm_client"
  - "LlmClient"
  - "CompletionRequest"
  - "DisabledLlmClient"
  - "OllamaAdapter"
  - "OLLAMA_URL"
  - "cloud_escalation_allowed"
  - "ModelTier"
  - "swap_model"
- RUN_COMMANDS:
  ```bash
  rg -n "trait LlmClient|CompletionRequest|swap_model" src/backend/handshake_core/src/llm -S
  rg -n "OLLAMA_URL|OLLAMA_MODEL|init_llm_client" src/backend/handshake_core/src/main.rs -S
  rg -n "cloud_escalation_allowed" src/backend/handshake_core/src/workflows.rs -S
  ```
- RISK_MAP:
  - "Secrets leak" -> "Cloud provider API keys appear in logs/artifacts/Flight Recorder; violates safety policy."
  - "SSRF / untrusted base_url" -> "Adapter can be abused to call internal services; must be policy/consent gated."
  - "Policy bypass" -> "Cloud tier invoked even when disallowed; must hard-block with deterministic error."

## SKELETON
- Proposed interfaces/types/contracts (no logic; names provisional):
  - `src/backend/handshake_core/src/llm/registry.rs`
    - `enum ProviderKind { Ollama, OpenAiCompat }`
    - `enum RuntimeRole { Frontend, Orchestrator, Worker, Validator }`
    - `struct ProviderRecord { provider_id, kind, tier: ModelTier, base_url, default_model_id, api_key_env?: String }`
    - `struct RoleAssignment { role: RuntimeRole, provider_id, model_id }`
    - `struct ResolvedProvider { provider_id, kind, tier: ModelTier, base_url, model_id }`
    - `struct ProviderRegistry { providers: BTreeMap<String, ProviderRecord>, assignments: BTreeMap<RuntimeRole, RoleAssignment> }`
    - `impl ProviderRegistry { fn from_env() -> Result<Self, LlmError>; fn resolve(&self, role: RuntimeRole) -> Result<ResolvedProvider, LlmError> }`
  - `src/backend/handshake_core/src/llm/openai_compat.rs`
    - `struct OpenAiCompatAdapter { base_url, profile: ModelProfile, client: reqwest::Client, flight_recorder, api_key: ApiKey(redacted) }`
    - Minimal OpenAI-compatible `/v1/chat/completions` request/response structs (prompt -> single user message; extract first choice text; map usage)
    - Emits `FlightRecorderEventType::LlmInference` with `trace_id`, `model_id`, token usage, latency, prompt_hash/response_hash; no raw prompt/payload.
  - `src/backend/handshake_core/src/llm/guard.rs`
    - `enum RuntimeGovernanceMode { Locked, GovStrict, GovStandard, GovLight }`
    - `struct CloudEscalationPolicy { governance_mode: RuntimeGovernanceMode, cloud_escalation_allowed: bool }`
      - Source of truth (v1): env vars loaded at backend startup (default-deny):
        - `HANDSHAKE_GOVERNANCE_MODE` in {`locked`, `gov_strict`, `gov_standard`, `gov_light`}; default: `gov_standard`
        - `HANDSHAKE_CLOUD_ESCALATION_ALLOWED` in {`true`, `false`}; default: `false`
      - Spec mapping: `RuntimeGovernanceMode::Locked` is the spec’s GovernanceMode `LOCKED` (cloud escalation MUST be denied).
    - Consent artifacts (v1):
      - `ProjectionPlanV0_4` (`hsk.projection_plan@0.4`) + `ConsentReceiptV0_4` (`hsk.consent_receipt@0.4`) loaded from env vars:
        - `HANDSHAKE_CLOUD_PROJECTION_PLAN_JSON`
        - `HANDSHAKE_CLOUD_CONSENT_RECEIPT_JSON`
      - If either is missing, Cloud tier calls hard-block (no network call attempted).
      - Binding enforcement (spec): `ConsentReceipt.payload_sha256` MUST equal `ProjectionPlan.payload_sha256` and `ConsentReceipt.projection_plan_id` MUST match.
      - Payload hash model (v1): `payload_sha256 = sha256(CompletionRequest.prompt UTF-8 bytes)`; no raw prompt logged.
    - `struct CloudEscalationGuard { inner: Arc<dyn LlmClient>, policy: CloudEscalationPolicy, consent: Option<CloudConsentArtifacts> }`
      - Enforcement order for `ModelTier::Cloud`:
        1) If `policy.governance_mode == Locked` => deny
        2) If `policy.cloud_escalation_allowed == false` => deny
        3) If consent artifacts missing => deny
        4) If consent artifacts do not bind / hash mismatch => deny
        5) Else => allow call to inner client
      - Consent is required for *any* cloud escalation (no exceptions) per spec.
  - `src/backend/handshake_core/src/main.rs`
    - Replace Ollama-only startup wiring with ProviderRegistry resolution (initially one resolved role used for backend runtime; structure supports per-role later).
    - Base URL validation helper for OpenAI-compatible provider (SSRF: treat env-provided base_url as untrusted; reject obviously unsafe hosts for Cloud tier unless policy explicitly allows).
  - Tests (offline):
    - `ProviderRegistry::from_env` deterministic resolution tests (incl. per-role overrides)
    - `OpenAiCompatAdapter` completion tested against local axum mock server (no real network)
    - Cloud policy guard test: when disallowed, adapter returns deterministic error and mock server sees 0 requests
    - Consent guard tests:
      - Missing artifacts => deterministic deny
      - Mismatched/bad artifacts => deterministic deny

- Open questions / assumptions:
  - NONE (All security-critical governance/consent behaviors are pinned for v1. Future wiring to Work Profiles/runtime governance can refine policy sources without changing enforcement semantics.)

- Notes:
  - No secrets (API keys) logged or stored in Flight Recorder/debug bundles; API key only read from env at runtime and held in-memory with redacted Debug/Display.
  - All base_url strings trimmed of trailing `/`; do not allow embedding credentials in URL.
  - Error codes (typed, deterministic) returned by the new guard/registry surfaces (modeled as `LlmError` variants in `src/backend/handshake_core/src/llm/mod.rs`):
    - `HSK-400-INVALID-BASE-URL`: base_url parse/scheme/credentials invalid
    - `HSK-403-SSRF-BLOCKED`: base_url host is disallowed for Cloud tier (loopback/private/link-local/etc.)
    - `HSK-403-CLOUD-ESCALATION-DENIED`: cloud escalation disallowed by policy
    - `HSK-403-GOVERNANCE-LOCKED`: governance mode is LOCKED => cloud escalation denied
    - `HSK-403-CLOUD-CONSENT-REQUIRED`: consent artifacts missing (ProjectionPlan + ConsentReceipt)
    - `HSK-403-CLOUD-CONSENT-MISMATCH`: consent receipt does not bind to projection plan or payload hash mismatch

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: server->network (LLM provider HTTP); env/config inputs are untrusted; cloud export requires explicit policy/consent.
- SERVER_SOURCES_OF_TRUTH:
  - Provider selection/config resolved by ProviderRegistry (server-side; deterministic env inputs; base_url validated)
  - Cloud escalation policy (server-side; default-deny; governance LOCKED denies)
  - Consent artifacts (ProjectionPlan + ConsentReceipt) (server-side; required for any Cloud tier call)
  - Flight Recorder events emitted by adapter (server-derived model/provider identity; do not trust client provenance)
- REQUIRED_PROVENANCE_FIELDS:
  - `trace_id` (required), `provider_id`, `model_id`, `model_tier`, `token_usage`, `latency_ms`, `prompt_hash`, `response_hash`
- VERIFICATION_PLAN:
  - Unit tests for ProviderRegistry resolution determinism and per-role mapping
  - Integration test with local mock HTTP server for OpenAI-compatible adapter
  - Guard test proving cloud-tier calls are blocked when policy disallows (no outbound HTTP attempted)
  - Regression check: Flight Recorder payload contains hashes/usage but no raw prompt or secrets
- ERROR_TAXONOMY_PLAN:
  - Invalid base_url: `LlmError::InvalidBaseUrl` => `HSK-400-INVALID-BASE-URL`
  - SSRF blocked: `LlmError::SsrBlocked` => `HSK-403-SSRF-BLOCKED`
  - Governance LOCKED: `LlmError::GovernanceLocked` => `HSK-403-GOVERNANCE-LOCKED`
  - Cloud policy disallowed: `LlmError::CloudEscalationDenied` => `HSK-403-CLOUD-ESCALATION-DENIED`
  - Consent missing: `LlmError::CloudConsentRequired` => `HSK-403-CLOUD-CONSENT-REQUIRED`
  - Consent mismatch: `LlmError::CloudConsentMismatch` => `HSK-403-CLOUD-CONSENT-MISMATCH`
  - Provider/network errors: `LlmError::ProviderError(...)` (retains HSK-500-LLM envelope; message includes stable sub-code where applicable)
- UI_GUARDRAILS:
  - Out of scope (no UI changes); backend returns deterministic errors for surfaces to render/handle later.
- VALIDATOR_ASSERTIONS:
  - Spec anchors satisfied: `LlmClient` invariants (Â§4.2.3), WorkProfile cloud disable honored (Â§4.3.7 local_only + allow_cloud_escalation), consent rules enforced/hard-blocked for Cloud tier as wired (Â§11.1.7).
  - RED_TEAM_ADVISORY enforced: no secrets in logs/FR; SSRF-safe base_url handling; no cloud calls when policy disallows.

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: BOOTSTRAP complete; SKELETON drafted (awaiting approval)
- What changed in this update: Claimed CODER_MODEL + reasoning strength; set packet status to In Progress; drafted SKELETON + E2E closure plan for review.
- Next step / handoff hint: Please reply with "SKELETON APPROVED" (or requested changes). After approval, I will implement provider registry + OpenAI-compatible adapter + policy guard + offline tests within IN_SCOPE_PATHS.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-LLM-Provider-Registry-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
