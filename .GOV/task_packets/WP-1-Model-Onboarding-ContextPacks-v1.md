# Task Packet: WP-1-Model-Onboarding-ContextPacks-v1

## METADATA
- TASK_ID: WP-1-Model-Onboarding-ContextPacks-v1
- WP_ID: WP-1-Model-Onboarding-ContextPacks-v1
- BASE_WP_ID: WP-1-Model-Onboarding-ContextPacks (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-12T03:19:22.509Z
- MERGE_BASE_SHA: 4618ed73838c01071f766c19721fc33534d6db4f
- REQUESTOR: ilja (Operator)
- AGENT_ID: CodexCLI-GPT-5.2
- ROLE: Orchestrator
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL: N/A
- ORCHESTRATION_STARTED_AT_UTC: N/A
- CODER_MODEL: GPT-5.2
- CODER_REASONING_STRENGTH: HIGH (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** In Progress
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
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - .GOV/roles_shared/TASK_BOARD.md
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
just post-work WP-1-Model-Onboarding-ContextPacks-v1 --range 4618ed73838c01071f766c19721fc33534d6db4f..HEAD
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
### Proposed contracts (no logic)
- `ContextPackPayloadV1` (spec 2.6.6.7.14.7): canonical JSON payload persisted in `pack_artifact`
  - `synopsis: string` (<= 800 chars)
  - `facts[]: {fact_id, text, source_refs[]: SourceRef, confidence: float}`
  - `constraints[]: {constraint_id, text, source_refs[]: SourceRef, severity: (hard|soft)}`
  - `open_loops[]: {loop_id, question, source_refs[]: SourceRef}`
  - `anchors[]: {anchor_id, source_ref: SourceRef, excerpt_hint: string}`
  - `coverage: {scanned_selectors[], skipped_selectors[]?}`
  - Phase 1 minimum useful builder output: `synopsis` + `anchors[]` + `coverage` (other arrays may be empty).
  - Builder output should emit the full schema surface with canonical JSON (stable hashes across runs); prefer anchors for span extraction when present (spec 2.6.6.7.14.6(G)).
- `ContextPackRecord` staleness semantics (to implement in `src/backend/handshake_core/src/ace/mod.rs`)
  - Replace order-dependent `source_hashes: Vec<Hash>` with deterministic `source_refs: Vec<SourceRef>` (sorted by `source_id`), OR explicitly define and enforce a stable ordering rule so staleness comparison is order-independent.
  - `is_stale(current: &[SourceRef]) -> bool` compares `source_id -> source_hash` deterministically.
- `ContextPackKeyV1` (deterministic lookup key; artifact-backed)
  - Inputs: `target` (`SourceRef`), `policy_id`, builder {tool_id, tool_version, config_hash}, and `sources_hash` (hash over sorted `(source_id, source_hash)`).
  - Output: stable string key (used for on-disk artifact path + retrieval).
- `ContextPackStore` (artifact IO surface; no `.GOV/` reads)
  - `load(key) -> Option<(ContextPackRecord, ContextPackPayloadV1)>`
  - `write(key, record, payload) -> (record_ref, payload_ref, payload_hash)`
  - Storage location (proposed): `data/context_packs/` (repo-relative).
- `ContextPackFreshnessPolicyV1` (policy knobs; stored/traceable)
  - `regen_allowed: bool` (effective; regeneration is capability/policy/consent-gated and may be denied)
  - `regen_required: bool` (if stale and regeneration not performed, `ContextPackFreshnessGuard` may fail; otherwise fallback + degraded)
- `ContextPackFreshnessDecision` (captured in trace + artifacts)
  - `Fresh { pack_id }`
  - `Stale { pack_id, reason }` (records warning marker `stale_pack:<pack_id>`)
  - `Regenerated { old_pack_id, new_pack_id }`

### Runtime behavior (spec-bound)
- Routing: attempt `StoreKind::ContextPacks` first; on fresh pack, select evidence using pack anchors/spans and set `CandidateScores.pack = Some(1.0)`.
- Freshness: if any underlying source hash differs at retrieval time, the pack is stale and MUST NOT be treated as `pack_score=1.0`; runtime deterministically:
  - if `policy.regen_allowed`: attempt regeneration; on success record `Regenerated` and allow `pack_score=1.0`
  - if regeneration is not allowed OR not performed:
    - if `policy.regen_required`: `ContextPackFreshnessGuard` returns an error (or marks the run degraded+failed) and emits `regen_skipped:<pack_id>`
    - otherwise: fall back to non-pack retrieval (e.g., ShadowWsLexical/ShadowWsVector), emit `stale_pack:<pack_id>`, and mark degraded/warn.
- Provenance binding: pack builder enforces `source_refs[]` on every fact/constraint/open_loop:
  - missing `source_refs[]` => item is dropped or forced `confidence=0` (deterministic)
  - such items MUST NOT be promoted to LongTermMemory.
- Trace markers used by `ContextPackFreshnessGuard`:
  - `stale_pack:<pack_id>`
  - `regen_skipped:<pack_id>` only when regeneration was required by policy but not performed.

- ### ModelSwapRequest fresh context compile boundary (spec 4.3.3.4.3)
- `ModelSwapRequestV0_4.context_compile_ref` is treated as a required, auditable "fresh context compilation" boundary artifact.
- On swap completion: write `context_compile_*.json` only after producing a fresh context snapshot for the target model; payload includes:
  - `context_snapshot_ref` + `context_snapshot_hash`
  - optional ContextPack references + hashes (if packs are used for compilation)
  - freshness decision markers (stale/regenerated/fallback) when applicable.

### Risks (high impact)
- Order-dependent `source_hashes[]` causes false staleness or missed drift; staleness must be keyed by `source_id`.
- Non-canonical JSON hashing produces unstable pack hashes across environments/serde versions.
- Regen loops/thrash on repeated staleness; must be bounded and deterministic, with a defined fallback.
- Provenance loss: pack items without `source_refs[]` accidentally promoted; must be filtered/downgraded at build time.
- Swap drift: model swap completes without a fresh compile artifact binding; must be enforced in the swap path.

### Decisions (resolved)
- ContextPack target: persisted packs are keyed by `SourceRef` (per-file / per-source). Per-run context compilation remains an ephemeral snapshot (not a persisted ContextPack) unless caching proves necessary.
- Freshness policy: stale packs MUST NOT score `1.0`; runtime regenerates if `regen_allowed`, otherwise falls back. Policy exposes `regen_allowed` and `regen_required`.
- Regen gating: regeneration is capability/policy/consent-gated (not always permitted). When regen is denied/skipped, runtime follows the same stale policy path (fallback by default, or error if `regen_required=true`).
- Payload minimum (Phase 1): implement the full `ContextPackPayloadV1` schema surface as canonical JSON; arrays may be empty. Minimum useful builder output is `synopsis` + `anchors[]` + `coverage`.
- Stale default: fallback + warnings/degraded (do not block the job) unless `regen_required=true`, in which case `ContextPackFreshnessGuard` may fail if regen is not performed.

### END_TO_END_CLOSURE_PLAN (summary) [CX-E2E-001]
- Trust boundary: server/runtime derives current `SourceRef` hashes and verifies artifacts; does not trust client-supplied provenance.
- Source of truth: `ContextPackStore` artifacts + server-derived `sources_hash` comparisons; decisions are traceable via `ContextPackFreshnessDecision` + markers.
- Audit linkage: ModelSwap context compilation (`context_compile_ref`) records snapshot refs/hashes and any ContextPack refs/hashes used.

### Open questions (remaining)
- None.

### Notes
- No product code changes until "SKELETON APPROVED".

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: server/runtime -> ContextPackStore (artifact IO) + ModelSwap `context_compile_ref` artifact
- SERVER_SOURCES_OF_TRUTH:
  - Current `SourceRef` hashes derived by the server from source bytes (not client supplied).
  - ContextPack artifacts loaded from `ContextPackStore` and verified by `sources_hash` + `payload_hash` (when present).
  - `context_compile_ref` artifact written and verified by the server during ModelSwap context compilation.
- REQUIRED_PROVENANCE_FIELDS:
  - `ContextPackKeyV1` inputs (target `SourceRef`, `policy_id`, builder id/version/config_hash, `sources_hash`).
  - `ContextPackRecord.source_refs[]` (sorted) + `sources_hash` + `ContextPackFreshnessDecision`.
  - `ContextPackPayloadV1` `synopsis` + `anchors[]` + `coverage` (and `facts/constraints/open_loops` when present), with `source_refs[]` on every item.
  - `payload_hash` + artifact refs (`record_ref`/`payload_ref`) + stale/regenerated/fallback markers + reason.
  - ModelSwap compile artifact: `context_snapshot_ref/hash` + optional ContextPack refs/hashes + freshness markers.
- VERIFICATION_PLAN:
  - On load: derive current `SourceRef` hashes; compute `sources_hash`; compare to record; stale => do not allow `pack_score=1.0`.
  - If stale and `regen_allowed`: regenerate and persist new record/payload; emit `Regenerated` marker.
  - If stale and regen is not performed:
    - if `regen_required`: error (or mark run degraded+failed) and emit `regen_skipped:<pack_id>`.
    - else: fall back retrieval + degraded/warn and emit `stale_pack:<pack_id>`.
  - For every payload item: enforce non-empty `source_refs[]`; drop/downgrade deterministically when missing.
  - For model swap: write `context_compile_*.json` only after producing a fresh context snapshot; include hashes/refs.
- ERROR_TAXONOMY_PLAN:
  - `ContextPackStale` (hash mismatch) vs `ContextPackArtifactCorrupt` (parse/hash mismatch).
  - `ContextPackRegenDenied` (policy) vs `ContextPackRegenFailed` (builder error).
  - `ContextPackProvenanceMissing` (item missing `source_refs[]`).
  - `ModelSwapCompileMissingOrStale` (missing compile artifact or stale marker present when regen required).
- UI_GUARDRAILS:
  - Surface stale/regenerated/fallback markers in job/operator output; treat stale as degraded by default (no job block).
  - When `regen_required=true`, surface as a hard error to prevent silent stale reuse.
- VALIDATOR_ASSERTIONS:
  - Stale packs never produce `pack_score=1.0` and always follow policy (regen or fallback or error).
  - Payload artifacts are canonical JSON and hash-verified; record binds `source_refs[]` deterministically.
  - Missing `source_refs[]` items are not promoted (drop/downgrade) and are traceable.
- ModelSwap compile artifact includes context snapshot refs/hashes and any ContextPack refs/hashes + freshness markers.

SKELETON APPROVED

## IMPLEMENTATION
- Added ContextPack schema surface (ContextPackRecord.source_hashes + source_refs; ContextPackPayloadV1 with synopsis/facts/constraints/open_loops/anchors/coverage) and provenance binding enforcement.
- Implemented SourceRef-keyed ContextPack storage under `data/context_packs/<source_id>/<builder_config_hash>/` with canonical JSON payload hashing and record/payload verification.
- Added ContextPack regeneration gating (policy + human consent + capability profile) and deterministic stale handling (regen if allowed, else fallback; emit stale_pack:/regen_skipped: markers when applicable).
- Extended MT context compilation to prefer ContextPacks (anchors-first span extraction) with deterministic fallback to ShadowWsLexical; ensures stale packs never yield pack_score=1.0.
- Implemented ModelSwap context compile artifact writer that records snapshot refs/hashes, ContextPack refs/hashes, and freshness markers.

## HYGIENE
- Logged decision follow-ups in refinement + traceability registry.
- Commands and logs are recorded in `## EVIDENCE` (logs in `.handshake/logs/`).

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/ace/mod.rs`
- **Start**: 855
- **End**: 1350
- **Line Delta**: 182
- **Pre-SHA1**: `df0cde80fca24dac5d1341c30c2a4b1436abf87b`
- **Post-SHA1**: `effe2e46c1e514ac8ec8a5a1626903316fba085a`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.126.md
- **Notes**:
 
- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 4
- **End**: 6927
- **Line Delta**: 970
- **Pre-SHA1**: `081288dd934723381c8394e40f4ec7eecb30161a`
- **Post-SHA1**: `0eb2011f8e029e15b84b1eac53b414c46a1096bd`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.126.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress
- What changed in this update:
  - Implemented SourceRef-keyed ContextPacks retrieval + freshness policy (regen if allowed, else fallback; regen is capability/policy/consent-gated).
  - Extended ModelSwap flow to record a context compile artifact that references snapshot + ContextPacks + freshness markers.
  - Updated refinement + WP traceability registry to reflect regen gating decision and status.
- Next step / handoff hint:
  - Validator review: confirm stale marker behavior + provenance binding match spec anchors and that regen is consent/capability/policy gated.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`
 
  - REQUIREMENT: "ContextPacks are treated as policy-bearing artifacts: ContextPackRecord.source_hashes[] freshness is enforced and stale packs deterministically trigger regeneration or fallback (covered by tests)."
  - EVIDENCE: `src/backend/handshake_core/src/ace/mod.rs:885`
 
  - REQUIREMENT: "ContextPack provenance binding is enforced: pack items without source_refs[] are dropped or deterministically downgraded (covered by tests)."
  - EVIDENCE: `src/backend/handshake_core/src/ace/mod.rs:993`
 
  - REQUIREMENT: "Stale packs MUST NOT be treated as pack_score=1.0. The runtime MUST either regenerate the pack (if allowed), or fall back to non-pack retrieval routes."
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:4188`
 
  - REQUIREMENT: "Stale packs never produce pack_score=1.0 and always follow policy (regen or fallback or error)."
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:6523`
 
  - REQUIREMENT: "Model swaps enforce fresh context compilation: ModelSwapRequest flow binds/records a context_compile_ref and does not reuse stale packs."
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:6914`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Model-Onboarding-ContextPacks-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`
 
  - COMMAND: `set "CARGO_INCREMENTAL=0" && set "RUSTFLAGS=-C debuginfo=0" && cargo test --manifest-path src/backend/handshake_core/Cargo.toml ace -j 1`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/cargo_test_ace_j1_debuginfo0_20260214_215315.log`
 
  - COMMAND: `set "CARGO_INCREMENTAL=0" && set "RUSTFLAGS=-C debuginfo=0" && cargo test --manifest-path src/backend/handshake_core/Cargo.toml -j 1`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/cargo_test_full_j1_debuginfo0_20260214_215343.log`
 
  - COMMAND: `just product-scan`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/just_product-scan_20260214_215845.log`
 
  - COMMAND: `just cargo-clean`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/just_cargo-clean_20260214_215901.log`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
