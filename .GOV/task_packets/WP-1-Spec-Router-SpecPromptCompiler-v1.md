# Task Packet: WP-1-Spec-Router-SpecPromptCompiler-v1

## METADATA
- TASK_ID: WP-1-Spec-Router-SpecPromptCompiler-v1
- WP_ID: WP-1-Spec-Router-SpecPromptCompiler-v1
- BASE_WP_ID: WP-1-Spec-Router-SpecPromptCompiler
- DATE: 2026-02-27T11:05:32.117Z
- MERGE_BASE_SHA: c01ddc665b32762ddefa8719037261afa1d96c18
- MERGE_BASE_SHA_NOTE: git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence
- REQUESTOR: Operator (ilja)
- AGENT_ID: user_orchestrator
- ROLE: Orchestrator
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL: N/A (AGENTIC_MODE=NO)
- ORCHESTRATION_STARTED_AT_UTC: N/A (AGENTIC_MODE=NO)
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja270220261121
- PACKET_FORMAT_VERSION: 2026-02-01

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: DISALLOWED
- OPERATOR_APPROVAL_EVIDENCE: N/A
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Spec-Router-SpecPromptCompiler-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement deterministic SpecPromptPack loading + SpecPromptCompiler for `spec_router`, producing reproducible PromptEnvelope hashes and ContextSnapshot lineage and recording/persisting provenance required by the Master Spec.
- Why: Prompt-to-spec routing must be replayable and auditable; deterministic compilation + hash/provenance capture prevents silent prompt drift and enables Validator-grade evidence.
- IN_SCOPE_PATHS:
  - assets/spec_prompt_packs/spec_router_pack@1.json
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/models.rs
  - src/backend/handshake_core/src/tokenization.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/spec_router/mod.rs (new)
  - src/backend/handshake_core/src/spec_router/spec_prompt_pack.rs (new)
  - src/backend/handshake_core/src/spec_router/spec_prompt_compiler.rs (new)
- OUT_OF_SCOPE:
  - CapabilitySnapshot generation rules and allowlist enforcement details (see WP-1-Spec-Router-CapabilitySnapshot-v1)
  - SpecLint engine + G-SPECLINT preflight gate (see WP-1-Spec-Router-SpecLint-v1)
  - Non-`spec_router` prompt compiler expansion (separate packets)
  - UI-only work beyond making required provenance fields present in Flight Recorder event payloads (separate packet)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Spec-Router-SpecPromptCompiler-v1
# ...task-specific commands...

cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml
cargo test --manifest-path src/backend/handshake_core/Cargo.toml
just cargo-clean
just post-work WP-1-Spec-Router-SpecPromptCompiler-v1 --range c01ddc665b32762ddefa8719037261afa1d96c18..HEAD
```

### DONE_MEANS
- SpecPromptPack asset exists at `assets/spec_prompt_packs/spec_router_pack@1.json` and includes the required fields (schema_version, pack_id, target_job_kind, stable_prefix_sections, variable_suffix_template_md, placeholders with max_tokens, required_outputs, budgets) per Master Spec 2.6.8.5.2.
- A deterministic `SpecPromptCompiler` exists and, for `spec_router`, it:
  - loads the selected pack (default `spec_router_pack@1`) and computes `spec_prompt_pack_id` and `spec_prompt_pack_sha256` (hash of the exact JSON bytes),
  - compiles PromptEnvelope such that stable_prefix is the concatenation of stable_prefix_sections in order, and variable_suffix is deterministic template expansion with deterministic truncation rules,
  - uses TokenizationService to enforce placeholder token caps and the envelope total budget, and records token counts and truncation flags.
- For each `spec_router` job, the system emits a ContextSnapshot that includes at minimum `prompt_ref` (handle + hash), `capability_snapshot_ref` (handle + hash), and `spec_prompt_pack_id` + `spec_prompt_pack_sha256` (Master Spec 2.6.8.5.2).
- For each `spec_router` job, Flight Recorder records and Operator Consoles can display the required provenance fields:
  - `spec_prompt_pack_id`, `spec_prompt_pack_sha256`, `context_snapshot_id`,
  - `prompt_envelope.stable_prefix_hash`, `prompt_envelope.variable_suffix_hash`,
  - token counts for stable_prefix + variable_suffix (and truncation flags, if any) (Master Spec 2.6.8.5.2 / 2.6.8.9).
- These provenance fields are copied into SpecIntent and SpecRouterDecision (see Master Spec 2.6.8.5 schemas referenced by 2.6.8.5.2).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.139.md (recorded_at: 2026-02-27T11:05:32.117Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 2.6.8.5.2 (SpecPromptPack + SpecPromptCompiler) [ADD v02.139]; Handshake_Master_Spec_v02.139.md 2.6.8.9 (Integration Hooks) [ADD v02.139]
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.139.md
  - assets/spec_prompt_packs/spec_router_pack@1.json
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/models.rs
  - src/backend/handshake_core/src/tokenization.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/capabilities.rs
- SEARCH_TERMS:
  - "SpecPromptPack"
  - "SpecPromptCompiler"
  - "SpecIntent"
  - "SpecRouterDecision"
  - "spec_prompt_pack_sha256"
  - "PromptEnvelope"
  - "ContextSnapshot"
  - "stable_prefix_hash"
  - "variable_suffix_hash"
  - "spec_router"
- RUN_COMMANDS:
  ```bash
  rg -n "spec_router" src/backend/handshake_core/src
  rg -n "SpecIntent|SpecRouterDecision|SpecPromptPack|SpecPromptCompiler" src/backend/handshake_core/src
  ```
- RISK_MAP:
  - "pack drift" -> "silent behavior change; mitigated by pack sha256 + provenance persistence"
  - "non-deterministic truncation/tokenization" -> "replay breaks; mitigated by deterministic truncation rules + token counts + flags"
  - "capability hallucination" -> "spec references nonexistent tools/engines; coordinated with CapabilitySnapshot allowlist WP"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: spec_router inputs (prompt_ref + assets + capability snapshot) -> compiled PromptEnvelope -> LLM call
- SERVER_SOURCES_OF_TRUTH:
  - Load SpecPromptPack from `assets/spec_prompt_packs/` and hash exact JSON bytes.
  - Load prompt content via `prompt_ref` artifact handle + hash (no inline prompt trust).
- REQUIRED_PROVENANCE_FIELDS:
  - spec_prompt_pack_id
  - spec_prompt_pack_sha256
  - capability_snapshot_ref (handle + hash)
  - context_snapshot_id
  - prompt_envelope.stable_prefix_hash
  - prompt_envelope.variable_suffix_hash
  - token counts + truncation flags
- VERIFICATION_PLAN:
  - Add tests that identical inputs produce identical PromptEnvelope hashes and identical provenance fields.
  - Add tests that token caps are enforced via TokenizationService and truncation flags are recorded when triggered.
- ERROR_TAXONOMY_PLAN:
  - missing_pack (pack_id not found)
  - pack_sha_mismatch (hash mismatch between recorded and loaded pack bytes)
  - budget_exceeded (total tokens > max_total_tokens after truncation attempt)
  - missing_required_placeholder (required placeholder missing)
- UI_GUARDRAILS:
  - N/A (no UI changes in this packet beyond ensuring required provenance fields are present in Flight Recorder payloads)
- VALIDATOR_ASSERTIONS:
  - For `spec_router` jobs, required provenance fields exist in Flight Recorder payload and in SpecIntent/SpecRouterDecision artifacts, and are consistent with computed hashes.

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
- Current WP_STATUS: In Progress
- What changed in this update: Claimed packet (CODER_MODEL/CODER_REASONING_STRENGTH) and moved Status -> In Progress.
- Next step / handoff hint: Fill `## SKELETON`, create docs-only skeleton checkpoint commit, and wait for "SKELETON APPROVED" before any product code changes.

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
  - LOG_PATH: `.handshake/logs/WP-1-Spec-Router-SpecPromptCompiler-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
