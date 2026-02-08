# Task Packet: WP-1-Product-Governance-Snapshot-v4

## METADATA
- TASK_ID: WP-1-Product-Governance-Snapshot-v4
- WP_ID: WP-1-Product-Governance-Snapshot-v4
- BASE_WP_ID: WP-1-Product-Governance-Snapshot (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-08T20:10:34.024Z
- MERGE_BASE_SHA: 0092ad1dcfec98e064f9eb97185ac493dedb7b42
- REQUESTOR: ilja (Operator) / Validator directive
- AGENT_ID: CodexCLI-GPT-5.2
- ROLE: Orchestrator
- AGENTIC_MODE: YES
- ORCHESTRATOR_MODEL: GPT-5.2 (Codex CLI) (required if AGENTIC_MODE=YES)
- ORCHESTRATION_STARTED_AT_UTC: 2026-02-08T20:10:34.024Z
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja080220262058
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Product-Governance-Snapshot-v4.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Remediate product/runtime governance boundary drift by removing Handshake runtime dependencies on repo governance paths (`docs/**` and `/.GOV/**`) and migrating runtime governance state to product-owned storage (default `.handshake/gov/`, configurable), while preserving the v3 Product Governance Snapshot requirements.
- Why: The repository governance workspace (`/.GOV/**`) and any `docs/**` compatibility bundle must not be runtime dependencies. This drift caused the product to rely on repo governance artifacts, confusing product vs repo governance and violating the intended boundary.
- IN_SCOPE_PATHS:
  - .GOV/task_packets/stubs/WP-1-Product-Governance-Snapshot-v1.md
  - .GOV/task_packets/WP-1-Product-Governance-Snapshot-v3.md
  - .GOV/refinements/WP-1-Product-Governance-Snapshot-v4.md
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - src/backend/handshake_core/src/governance_pack.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/api/role_mailbox.rs (if required by refactor)
  - src/backend/handshake_core/src/api/governance_pack.rs (if required by refactor)
  - affected Rust tests under `src/backend/handshake_core/src/**` and `src/backend/handshake_core/tests/**` (only if required by changes)
- OUT_OF_SCOPE:
  - Any product feature work unrelated to governance boundary/state paths
  - Refactoring `.GOV/**` workflow authoring beyond what is required to preserve v3 snapshot tooling
  - Syncing/merging branches with `origin/main` (explicit Operator auth required)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Product-Governance-Snapshot-v4

# Prove runtime no longer depends on repo governance paths (post-change):
rg -n \"docs/\" src/backend/handshake_core/src -S
rg -n \"\\\\.GOV/\" src/backend/handshake_core/src -S

# Backend verification:
cd src/backend/handshake_core; cargo test

just cargo-clean
just post-work WP-1-Product-Governance-Snapshot-v4 --range 0092ad1dcfec98e064f9eb97185ac493dedb7b42..HEAD
```

### DONE_MEANS
- Product runtime does not require repo `docs/**` for governance-critical behavior (no reads of `docs/SPEC_CURRENT.md`, `docs/TASK_BOARD.md`, or `docs/ROLE_MAILBOX/**` as runtime sources of truth).
- Product runtime does not read from or write to `/.GOV/**` (hard boundary preserved).
- Runtime governance state defaults to product-owned `.handshake/gov/` (configurable) and is used for any runtime governance state that previously lived in repo folders.
- Any compatibility `docs/**` outputs (if retained) are explicitly optional/compat-only and must not be required for runtime correctness.
- Tests updated so `cargo test` passes without requiring repo-governance `docs/**` inputs as authoritative state.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.125.md (recorded_at: 2026-02-08T20:10:34.024Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md `#### 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)` + `#### 7.5.4.10 Product Governance Snapshot (HARD)`
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- BASE_WP_ID: WP-1-Product-Governance-Snapshot
- v1 (STUB; do NOT activate/merge):
  - .GOV/task_packets/stubs/WP-1-Product-Governance-Snapshot-v1.md
  - Preserve (end-state intent): product runtime must not depend on repo `.GOV/**` or repo `docs/**`; governance defaults/templates shipped inside product; runtime governance state stored in product-owned storage (not repo folders); `docs/**` may exist as compatibility only (short-term).
- v3 (ACTIVE prior; implemented snapshot tooling):
  - .GOV/task_packets/WP-1-Product-Governance-Snapshot-v3.md
  - Preserve: v02.125 `#### 7.5.4.10` Product Governance Snapshot generator + validator requirements (determinism, leak-safety, canonical whitelist inputs, default output `.GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json`).
- v4 (THIS PACKET):
  - Change/add: explicitly carry forward the v1 decouple + product-owned runtime governance state boundary and apply it to the actual runtime implementation so the product no longer relies on repo governance paths in practice.
  - Rule: Do NOT resurrect/activate v1; do NOT merge abandoned v2/v3-docs attempts. Treat `docs/**` as compatibility-only if kept.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.125.md (anchors: 7.5.4.8, 7.5.4.10)
  - .GOV/task_packets/WP-1-Product-Governance-Snapshot-v3.md
  - .GOV/task_packets/stubs/WP-1-Product-Governance-Snapshot-v1.md
  - src/backend/handshake_core/src/governance_pack.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- SEARCH_TERMS:
  - "docs/SPEC_CURRENT.md"
  - "docs/TASK_BOARD.md"
  - "docs/ROLE_MAILBOX/"
  - "ROLE_MAILBOX_EXPORT_ROOT"
  - "export_root"
  - "spec_target_resolved"
- RUN_COMMANDS:
  ```bash
  rg -n \"docs/\" src/backend/handshake_core/src -S
  rg -n \"\\\\.GOV/\" src/backend/handshake_core/src -S
  ```
- RISK_MAP:
  - "path migration" -> "breaking runtime expectations for existing on-disk exports/state; requires explicit compatibility strategy"
  - "boundary regression" -> "runtime accidentally reintroduces repo `docs/**` or `/.GOV/**` reads/writes"
  - "leak risk" -> "role mailbox export/snapshot accidentally includes secrets/raw bodies"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: NO
- TRUST_BOUNDARY: N/A
- SERVER_SOURCES_OF_TRUTH:
  - N/A
- REQUIRED_PROVENANCE_FIELDS:
  - N/A
- VERIFICATION_PLAN:
  - N/A
- ERROR_TAXONOMY_PLAN:
  - N/A
- UI_GUARDRAILS:
  - N/A
- VALIDATOR_ASSERTIONS:
  - N/A

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
  - LOG_PATH: `.handshake/logs/WP-1-Product-Governance-Snapshot-v4/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
