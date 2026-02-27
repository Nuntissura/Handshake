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
  - src/backend/handshake_core/src/spec_router/mod.rs
  - src/backend/handshake_core/src/spec_router/spec_prompt_pack.rs
  - src/backend/handshake_core/src/spec_router/spec_prompt_compiler.rs
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
  - `assets/spec_prompt_packs/spec_router_pack@1.json`
    - `SpecPromptPackV1` JSON schema per Master Spec 2.6.8.5.2 (schema_version/pack_id/target_job_kind/stable_prefix_sections/variable_suffix_template_md/placeholders/required_outputs/budgets).
    - Pack hashing: `spec_prompt_pack_sha256 = SHA-256(exact JSON bytes on disk)` (no re-serialization).
    - Draft JSON (for SKELETON review; will be written to the file after "SKELETON APPROVED" unless explicitly overridden):
      ```json
      {
        "schema_version": "hsk.spec_prompt_pack@1",
        "pack_id": "spec_router_pack@1",
        "description": "Deterministic prompt envelope skeleton for Spec Router (Prompt->Spec).",
        "target_job_kind": "spec_router",
        "stable_prefix_sections": [
          {
            "section_id": "SYSTEM_RULES",
            "content_md": "## SYSTEM RULES (HARD)\n- You are running as Handshake Spec Router / Spec Author.\n- You MUST NOT invent tools, engines, surfaces, connectors, events, or files.\n- You MAY only reference items listed in CAPABILITY SNAPSHOT.\n- If you lack information, record assumptions as NEEDS_CONFIRMATION in ## Assumptions.\n- Output MUST follow OUTPUT CONTRACT exactly."
          },
          {
            "section_id": "OUTPUT_CONTRACT",
            "content_md": "## REQUIRED OUTPUTS (HARD)\nYou MUST output, in order:\n1) SpecIntent (JSON)\n2) SpecRouterDecision (JSON)\n3) Spec artifact (Markdown)\n\n## OUTPUT CONTRACT (STRICT)\n- No extra prose outside the three artifacts.\n- All IDs must be stable and machine-readable."
          }
        ],
        "variable_suffix_template_md": "## INPUTS\n### User prompt\n- prompt_ref: {{PROMPT_REF}}\n- prompt_text: {{PROMPT_TEXT}}\n\n### Workspace/workflow context\n- workspace_id: {{WORKSPACE_ID}}\n- project_id: {{PROJECT_ID}}\n- version_control: {{VERSION_CONTROL}}\n- repo_root: {{REPO_ROOT}}\n\n## CAPABILITY SNAPSHOT (ALLOWED ONLY)\n{{CAPABILITY_SNAPSHOT_TABLE}}\n\n## GOVERNANCE\n- governance_mode: {{GOVERNANCE_MODE}}\n- required_gates: {{REQUIRED_GATES}}\n\nBEGIN WORK:\n",
        "placeholders": [
          { "name": "PROMPT_REF", "source": "prompt_ref", "max_tokens": 32, "required": true },
          { "name": "PROMPT_TEXT", "source": "prompt_ref", "max_tokens": 900, "required": true },
          { "name": "WORKSPACE_ID", "source": "workflow_context", "max_tokens": 64, "required": true },
          { "name": "PROJECT_ID", "source": "workflow_context", "max_tokens": 64, "required": false },
          { "name": "VERSION_CONTROL", "source": "workflow_context", "max_tokens": 16, "required": true },
          { "name": "REPO_ROOT", "source": "workflow_context", "max_tokens": 256, "required": false },
          { "name": "CAPABILITY_SNAPSHOT_TABLE", "source": "capability_snapshot", "max_tokens": 900, "required": true },
          { "name": "GOVERNANCE_MODE", "source": "governance_mode", "max_tokens": 16, "required": true },
          { "name": "REQUIRED_GATES", "source": "governance_mode", "max_tokens": 128, "required": true }
        ],
        "required_outputs": [
          { "artifact_kind": "SpecIntent", "schema_ref": "hsk.spec_intent@0.2" },
          { "artifact_kind": "SpecRouterDecision", "schema_ref": "hsk.spec_router_decision@0.2" },
          { "artifact_kind": "SpecArtifact", "schema_ref": "hsk.feature_spec@0.2" }
        ],
        "budgets": {
          "max_total_tokens": 8000,
          "max_prompt_excerpt_tokens": 900,
          "max_capsule_tokens": 1200,
          "max_capability_table_tokens": 900
        }
      }
      ```
  - `src/backend/handshake_core/src/spec_router/mod.rs` (new)
    - `pub mod spec_prompt_pack;`
    - `pub mod spec_prompt_compiler;`
  - `src/backend/handshake_core/src/spec_router/spec_prompt_pack.rs` (new)
    - `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct SpecPromptPackV1 { schema_version, pack_id, description, target_job_kind, stable_prefix_sections, variable_suffix_template_md, placeholders, required_outputs, budgets }`
    - `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct StablePrefixSectionV1 { section_id: String, content_md: String }`
    - `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct PlaceholderV1 { name: String, source: PlaceholderSourceV1, max_tokens: u32, required: bool }`
    - `#[derive(Debug, Clone, Serialize, Deserialize)] pub enum PlaceholderSourceV1 { PromptRef, CapabilitySnapshot, WorkflowContext, GovernanceMode }` (serde snake_case)
    - `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct RequiredOutputV1 { artifact_kind: String, schema_ref: String }`
    - `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct BudgetsV1 { max_total_tokens: u32, max_prompt_excerpt_tokens: u32, max_capsule_tokens: u32, max_capability_table_tokens: u32 }`
    - `pub struct LoadedSpecPromptPack { pub pack: SpecPromptPackV1, pub pack_id: String, pub pack_sha256: String, pub raw_bytes: Vec<u8> }`
    - `pub fn load_spec_prompt_pack(pack_id: &str) -> Result<LoadedSpecPromptPack, SpecPromptPackError>`
  - `src/backend/handshake_core/src/spec_router/spec_prompt_compiler.rs` (new)
    - `pub struct WorkingContextV1 { pub blocks: Vec<ContextBlockV1>, pub token_budget: u32, pub token_estimate: u32, pub build_id: String }` (Master Spec 2.6.6.7.4)
    - `pub struct ContextBlockV1 { pub kind: String, pub content: String, pub source_refs: Vec<Value>, pub sensitivity: String, pub projection: String, pub order_key: String }` (Master Spec 2.6.6.7.4; arrays MUST be in deterministic order)
    - `pub struct PromptEnvelopeV1 { pub stable_prefix: WorkingContextV1, pub variable_suffix: WorkingContextV1, pub stable_prefix_hash: String, pub variable_suffix_hash: String, pub full_prompt_hash: String, pub stable_prefix_tokens: u32, pub variable_suffix_tokens: u32, pub total_tokens: u32, pub truncation: PromptEnvelopeTruncationV1 }`
    - `pub struct PromptEnvelopeTruncationV1 { pub per_placeholder_truncated: BTreeMap<String, bool>, pub variable_suffix_truncated: bool }`
    - `pub struct SpecPromptCompiler<'a> { pub tokenization: &'a dyn TokenizationService, pub model_id: &'a str }`
    - `pub fn compile_spec_router_envelope(pack: &SpecPromptPackV1, values: &BTreeMap<String, String>, tokenization: &dyn TokenizationService, model_id: &str) -> Result<PromptEnvelopeV1, SpecPromptCompilerError>`
    - Determinism rules (compiler):
      - `stable_prefix` MUST be a canonical WorkingContext with deterministic `blocks[]` (derived from `stable_prefix_sections` in order).
      - `variable_suffix` MUST be a canonical WorkingContext with deterministic `blocks[]` whose `content` is the deterministic template expansion of `variable_suffix_template_md`.
      - Placeholder enforcement order: (1) require presence for `required=true`, (2) truncate each placeholder to `max_tokens` via TokenizationService, (3) expand template, (4) enforce `budgets.max_total_tokens` by truncating `variable_suffix` to remaining tokens (record `variable_suffix_truncated=true`), else `budget_exceeded` if stable_prefix alone exceeds budget.
      - Hashes (Spec 2.6.6.7.4/2.6.6.7.5): SHA-256 over UTF-8 bytes of **canonical JSON** for WorkingContext blocks (sorted keys + NFC + deterministic arrays). Use `crate::llm::canonical_json_bytes_nfc` + `crate::llm::sha256_hex` (or `flight_recorder::canonical_json_sha256_hex`) to compute `stable_prefix_hash`, `variable_suffix_hash`, and `full_prompt_hash`.
  - `src/backend/handshake_core/src/workflows.rs` (integration points; implementation after skeleton approval)
    - Add `JobKind::SpecRouter` execution branch.
    - Parse `job_inputs` as `SpecRouterJobProfile` per Master Spec 2.6.6.6.5.
    - Load SpecPromptPack (default `spec_router_pack@1`) and compute `(spec_prompt_pack_id, spec_prompt_pack_sha256)`.
    - Load prompt bytes from `prompt_ref.path` (ArtifactHandle) and compute `prompt_sha256` for ContextSnapshot.
    - Generate/store CapabilitySnapshot artifact (minimal deterministic generation in this WP) and compute `capability_snapshot_ref + capability_snapshot_sha256` for ContextSnapshot (allowlist enforcement/rules remain OUT_OF_SCOPE here).
    - Compile PromptEnvelope via SpecPromptCompiler; record hashes, token counts, truncation flags.
    - Emit ContextSnapshot artifact (JSON) including at minimum: `prompt_ref + prompt_sha256`, `capability_snapshot_ref + capability_snapshot_sha256`, `spec_prompt_pack_id + spec_prompt_pack_sha256`, `context_snapshot_id`, and `prompt_envelope.{stable_prefix_hash,variable_suffix_hash}`.
    - Flight Recorder provenance: extend `llm_inference` payload with required fields (Spec 2.6.8.5.2 / 2.6.8.9) without trusting model-provided provenance.
    - SpecIntent + SpecRouterDecision artifacts: parse model output for semantic fields, but set/overwrite required provenance fields server-side from computed truth (including ContextSnapshot id, PromptEnvelope hashes, token counts, truncation flags).

- END_TO_END_CLOSURE_PLAN (SKELETON mirror; see `## END_TO_END_CLOSURE_PLAN [CX-E2E-001]` for full list):
  - Producer (server-derived truth): `SpecPromptPack` bytes->sha256; `SpecPromptCompiler` -> stable/variable strings + hashes + token counts + truncation flags; ContextSnapshot JSON -> id + artifact refs/hashes.
  - Transport: Flight Recorder `llm_inference` event payload MUST include `spec_prompt_pack_id`, `spec_prompt_pack_sha256`, `context_snapshot_id`, `prompt_envelope.stable_prefix_hash`, `prompt_envelope.variable_suffix_hash`, and token counts + truncation flags.
  - Persistence: write `context_snapshot.json` + `spec_intent.json` + `spec_router_decision.json` artifacts under a deterministic job directory; link handles in job output.
  - Trust boundary: job_inputs and model output are untrusted; provenance fields MUST be computed and enforced by the server.
  - Determinism: no randomness; identical pack bytes + prompt bytes + capability snapshot bytes + workflow context => identical hashes.

- Contract decisions (locked for implementation):
  - CapabilitySnapshot: MUST do minimal deterministic generation in this WP (emit artifact + hash + inject into prompt); allowlist enforcement/rules remain OUT_OF_SCOPE.
  - SpecRouterDecision provenance: implement per Spec 2.6.8.5.2 (include `context_snapshot_id`, PromptEnvelope hashes, token counts, truncation flags). Treat the 2.6.8.5 Rust snippet as incomplete if it omits these fields.
  - WorkingContext rendering: define deterministic text rendering for the LLM request while keeping hashes defined over canonical JSON:
    - `stable_prefix_text = join(stable_prefix.blocks[*].content, \"\\n\\n\")` in block order
    - `variable_suffix_text = join(variable_suffix.blocks[*].content, \"\\n\\n\")` in block order
    - `full_prompt_text = stable_prefix_text + \"\\n\\n\" + variable_suffix_text`

- Open questions:
  - Confirm `full_prompt_hash` computation target (canonical JSON of a `{stable_prefix, variable_suffix}` object vs canonical JSON of concatenated block list), consistent with Spec 2.6.6.7.3/2.6.6.7.4.

- Notes:
  - No product code changes until "SKELETON APPROVED" (per CODER_PROTOCOL [CX-GATE-001]).

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
- Added SpecPromptPack asset: `assets/spec_prompt_packs/spec_router_pack@1.json` (SpecPromptPackV1 fields per Master Spec 2.6.8.5.2).
- Added `spec_router` module:
  - `src/backend/handshake_core/src/spec_router/spec_prompt_pack.rs`: deterministic pack loader + exact-bytes SHA-256.
  - `src/backend/handshake_core/src/spec_router/spec_prompt_compiler.rs`: deterministic PromptEnvelope compilation, token caps, truncation flags, and canonical-json hashing over WorkingContext blocks.
- Updated `src/backend/handshake_core/src/models.rs`: added `SpecRouterJobProfile` + `WorkflowContext` + `VersionControl` (Master Spec 2.6.6.6.5).
- Updated `src/backend/handshake_core/src/workflows.rs`:
  - Routes `JobKind::SpecRouter` to `run_spec_router_job(...)`.
  - Loads SpecPromptPack (default `spec_router_pack@1`) + computes `spec_prompt_pack_sha256`.
  - Loads prompt bytes from `prompt_ref` + computes prompt SHA-256.
  - Generates minimal deterministic CapabilitySnapshot artifact (JSON) and injects a rendered capability table into the router prompt.
  - Compiles PromptEnvelope (stable_prefix + variable_suffix) via SpecPromptCompiler; records hashes, token counts, and truncation flags.
  - Emits ContextSnapshot artifact (JSON) including required provenance refs/hashes and SpecPromptPack id/sha.
  - Records required provenance fields into Flight Recorder LlmInference payload (server-computed).
  - Emits SpecIntent + SpecRouterDecision artifacts with required provenance fields set server-side (not trusted from model output).

## HYGIENE
- Gates run:
  - `just hard-gate-wt-001`
  - `just pre-work WP-1-Spec-Router-SpecPromptCompiler-v1`
- Deterministic manifest prep:
  - `git add` (stage only WP files)
  - `just cor701-sha <file>` for each changed non-`.GOV/` file

## VALIDATION
- (Deterministic manifest for `just post-work` / COR-701 checks; no verdicts.)

- **Target File**: `assets/spec_prompt_packs/spec_router_pack@1.json`
- **Start**: 1
- **End**: 39
- **Line Delta**: 39
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `ab406f08f2281d6da22ff2b91db626b66b48ddfd`
- **Gates Passed**:
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/src/models.rs`
- **Start**: 1
- **End**: 181
- **Line Delta**: 38
- **Pre-SHA1**: `31316f5d7276ab8603060156e35cfe0172197302`
- **Post-SHA1**: `4b1704483d3461e92b3dfed10f85bacc31dc9b46`
- **Gates Passed**:
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1
- **End**: 19422
- **Line Delta**: 788
- **Pre-SHA1**: `1cde8d5281ae7a9d22e03142a5bd16b3aa12eb3f`
- **Post-SHA1**: `6686cc63d670dc44966a2dc066e4ac2142fd5be8`
- **Gates Passed**:
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/src/spec_router/mod.rs`
- **Start**: 1
- **End**: 2
- **Line Delta**: 2
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `47a83f333c56ec4d7f75a58c6970be16a8a3bb64`
- **Gates Passed**:
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/src/spec_router/spec_prompt_pack.rs`
- **Start**: 1
- **End**: 148
- **Line Delta**: 148
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `f712fe145a47f51d7db3aeb71fe322e0db835b73`
- **Gates Passed**:
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/src/spec_router/spec_prompt_compiler.rs`
- **Start**: 1
- **End**: 248
- **Line Delta**: 248
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `8dfd67fa1f7ce1d9063ceee91b217f1f68545858`
- **Gates Passed**:
  - [x] all_links_resolvable

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress
- What changed in this update: Implemented SpecPromptPack loading + SpecPromptCompiler + spec_router integration and staged only WP in-scope files for deterministic post-work validation.
- Next step / handoff hint: Run `just post-work WP-1-Spec-Router-SpecPromptCompiler-v1` (staged diff mode) and then commit the staged WP files.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- REQUIREMENT: "SpecPromptPack asset exists at `assets/spec_prompt_packs/spec_router_pack@1.json` and includes the required fields per Master Spec 2.6.8.5.2."
  - EVIDENCE: `assets/spec_prompt_packs/spec_router_pack@1.json:1`
- REQUIREMENT: "Loads the selected pack (default `spec_router_pack@1`) and computes `spec_prompt_pack_id` and `spec_prompt_pack_sha256` (hash of the exact JSON bytes)."
  - EVIDENCE: `src/backend/handshake_core/src/spec_router/spec_prompt_pack.rs:121`
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:10732`
- REQUIREMENT: "Compiles PromptEnvelope such that stable_prefix is the concatenation of stable_prefix_sections in order, and variable_suffix is deterministic template expansion with deterministic truncation rules."
  - EVIDENCE: `src/backend/handshake_core/src/spec_router/spec_prompt_compiler.rs:140`
  - EVIDENCE: `src/backend/handshake_core/src/spec_router/spec_prompt_compiler.rs:156`
- REQUIREMENT: "Uses TokenizationService to enforce placeholder token caps and the envelope total budget, and records token counts and truncation flags."
  - EVIDENCE: `src/backend/handshake_core/src/spec_router/spec_prompt_compiler.rs:118`
  - EVIDENCE: `src/backend/handshake_core/src/spec_router/spec_prompt_compiler.rs:204`
- REQUIREMENT: "For each spec_router job, the system emits a ContextSnapshot that includes at minimum prompt_ref (handle + hash), capability_snapshot_ref (handle + hash), and spec_prompt_pack_id + spec_prompt_pack_sha256."
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:10525`
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:10957`
- REQUIREMENT: "For each spec_router job, Flight Recorder records and Operator Consoles can display the required provenance fields (pack id/sha, context_snapshot_id, prompt_envelope hashes, token counts, truncation flags)."
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11041`
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11063`
- REQUIREMENT: "These provenance fields are copied into SpecIntent and SpecRouterDecision."
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11083`
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11111`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `just hard-gate-wt-001`
  - EXIT_CODE: 0
  - COMMAND: `just pre-work WP-1-Spec-Router-SpecPromptCompiler-v1`
  - EXIT_CODE: 0
  - COMMAND: `git diff --cached --numstat -- assets/spec_prompt_packs/spec_router_pack@1.json src/backend/handshake_core/src/models.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/src/spec_router/mod.rs src/backend/handshake_core/src/spec_router/spec_prompt_pack.rs src/backend/handshake_core/src/spec_router/spec_prompt_compiler.rs`
  - EXIT_CODE: 0
  - COMMAND: `just cor701-sha src/backend/handshake_core/src/workflows.rs`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Spec-Router-SpecPromptCompiler-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
