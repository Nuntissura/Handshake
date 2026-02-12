# Task Packet: WP-1-Model-Onboarding-ContextPacks-v1

## METADATA
- TASK_ID: WP-1-Model-Onboarding-ContextPacks-v1
- WP_ID: WP-1-Model-Onboarding-ContextPacks-v1
- BASE_WP_ID: WP-1-Model-Onboarding-ContextPacks (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-12T03:19:22.509Z
- MERGE_BASE_SHA: fadbbeb81693b7aa82ecd7eb8eca78dfc28c0049
- REQUESTOR: ilja (Operator)
- AGENT_ID: CodexCLI-GPT-5.2
- ROLE: Orchestrator
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL: N/A
- ORCHESTRATION_STARTED_AT_UTC: N/A
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja120220260341
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Model-Onboarding-ContextPacks-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement deterministic model onboarding via ContextPacks (artifact-backed, hashable, provenance-bound) and enforce freshness + swap-safe context recompilation. Ensure stale ContextPacks are detected and the runtime deterministically regenerates or falls back per spec; ensure ModelSwapRequest enforces a fresh context compile boundary.
- Why: Without deterministic onboarding artifacts, multi-model execution is non-reproducible and governance constraints can drift across swaps/escalations; the Master Spec requires ContextPacks freshness/provenance rules and requires fresh context compilation on ModelSwapRequest resume.
- IN_SCOPE_PATHS:
  - .GOV/task_packets/WP-1-Model-Onboarding-ContextPacks-v1.md
  - .GOV/task_packets/stubs/WP-1-Model-Onboarding-ContextPacks-v1.md
  - .GOV/refinements/WP-1-Model-Onboarding-ContextPacks-v1.md
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/ace/validators/freshness.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/llm/mod.rs
- OUT_OF_SCOPE:
  - Provider registry / cloud adapters (handled by WP-1-LLM-Provider-Registry-v1)
  - UI polish for ContextPack inspection (Operator Consoles)
  - Any weakening of freshness/provenance enforcement (stale packs must be handled deterministically)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Model-Onboarding-ContextPacks-v1

# Targeted backend tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml ace

# Full backend tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Mechanical scan gates:
just product-scan

just cargo-clean
just post-work WP-1-Model-Onboarding-ContextPacks-v1 --range fadbbeb81693b7aa82ecd7eb8eca78dfc28c0049..HEAD
```

### DONE_MEANS
- ContextPacks are treated as policy-bearing artifacts: `ContextPackRecord.source_hashes[]` freshness is enforced and stale packs deterministically trigger regeneration or fallback (covered by tests).
- ContextPack provenance binding is enforced: pack items without `source_refs[]` are dropped or deterministically downgraded (covered by tests).
- Model swaps enforce fresh context compilation: ModelSwapRequest flow binds/records a `context_compile_ref` and does not reuse stale packs (covered by unit/integration test around swap path).
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` + `just product-scan` + `just post-work WP-1-Model-Onboarding-ContextPacks-v1 --range fadbbeb81693b7aa82ecd7eb8eca78dfc28c0049..HEAD` all PASS.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.125.md (recorded_at: 2026-02-12T03:19:22.509Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 2.6.6.7.14.7 (ContextPacks freshness + provenance binding) + 4.3.3.4 (ModelSwapRequest fresh context recompile requirement)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- BASE_WP_ID: WP-1-Model-Onboarding-ContextPacks
- v1 (THIS PACKET; activated from stub):
  - Stub source: .GOV/task_packets/stubs/WP-1-Model-Onboarding-ContextPacks-v1.md
  - Prior official packets: NONE
  - Purpose of v1: first activation into an executable packet with signed refinement + deterministic gates.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - .GOV/refinements/WP-1-Model-Onboarding-ContextPacks-v1.md
  - .GOV/task_packets/stubs/WP-1-Model-Onboarding-ContextPacks-v1.md
  - Handshake_Master_Spec_v02.125.md (anchors: 2.6.6.7.14.7, 4.3.3.4)
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/ace/validators/freshness.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/llm/mod.rs
- SEARCH_TERMS:
  - "ContextPackRecord"
  - "ContextPackFreshnessGuard"
  - "StoreKind::ContextPacks"
  - "source_hashes"
  - "source_refs"
  - "ModelSwapRequestV0_4"
  - "context_compile_ref"
  - "swap_model"
- RUN_COMMANDS:
  ```bash
  rg -n "ContextPackRecord|ContextPackFreshnessGuard|StoreKind::ContextPacks" src/backend/handshake_core/src/ace -S
  rg -n "ModelSwapRequestV0_4|context_compile_ref" src/backend/handshake_core/src/workflows.rs -S
  ```
- RISK_MAP:
  - "Stale packs treated as fresh" -> "Invalid/unsafe context is reused across runs; violates spec freshness rules."
  - "Constraint drift across swap" -> "ModelSwap loses governance constraints; must enforce fresh context compile boundary."
  - "Provenance loss" -> "Pack items without source_refs get promoted; must drop/downgrade deterministically."

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
  - LOG_PATH: `.handshake/logs/WP-1-Model-Onboarding-ContextPacks-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
