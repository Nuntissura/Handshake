# Task Packet: WP-1-LLM-Provider-Registry-v1

## METADATA
- TASK_ID: WP-1-LLM-Provider-Registry-v1
- WP_ID: WP-1-LLM-Provider-Registry-v1
- BASE_WP_ID: WP-1-LLM-Provider-Registry (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-12T03:19:16.812Z
- MERGE_BASE_SHA: fadbbeb81693b7aa82ecd7eb8eca78dfc28c0049 (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
- REQUESTOR: ilja (Operator)
- AGENT_ID: CodexCLI-GPT-5.2
- ROLE: Orchestrator
- AGENTIC_MODE: NO (YES | NO)
- ORCHESTRATOR_MODEL: N/A (required if AGENTIC_MODE=YES)
- ORCHESTRATION_STARTED_AT_UTC: N/A (RFC3339 UTC; required if AGENTIC_MODE=YES)
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
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
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES | NO
- TRUST_BOUNDARY: <fill> (examples: client->server, server->storage, job->apply)
- SERVER_SOURCES_OF_TRUTH:
  - <fill> (what the server loads/verifies instead of trusting the client)
- REQUIRED_PROVENANCE_FIELDS:
  - <fill> (role_id, contract_id, model_id/tool_id, evidence refs, before/after spans, etc.)
- VERIFICATION_PLAN:
  - <fill> (how provenance/audit is verified and recorded; include non-spoofable checks when required)
- ERROR_TAXONOMY_PLAN:
  - <fill> (distinct error classes: stale/mismatch vs spoof attempt vs true scope violation)
- UI_GUARDRAILS:
  - <fill> (prevent stale apply; preview before apply; disable conditions)
- VALIDATOR_ASSERTIONS:
  - <fill> (what the validator must prove; spec anchors; fields present; trust boundary enforced)

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
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

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

