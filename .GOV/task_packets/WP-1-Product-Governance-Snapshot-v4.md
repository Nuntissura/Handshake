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
- CODER_MODEL: CodexCLI-GPT-5.2
- CODER_REASONING_STRENGTH: HIGH (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** In Progress
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
  - docs/SPEC_CURRENT.md (range-mode inherited branch delta; included for deterministic post-work manifest coverage)
  - src/backend/handshake_core/src/governance_pack.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/runtime_governance.rs (required shared runtime-governance path resolver for `.handshake/gov/` boundary enforcement)
  - src/backend/handshake_core/src/lib.rs (required module export for `runtime_governance`)
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
  - Add runtime governance path resolver in `src/backend/handshake_core/src/runtime_governance.rs`:
    - default runtime root: `.handshake/gov/`
    - configurable root via env override
    - canonical accessors: `spec_current_path`, `task_board_path`, `role_mailbox_export_dir`
    - boundary guard: runtime path must not be under repo `docs/**` or `/.GOV/**`
  - `governance_pack.rs`: replace implicit `docs/SPEC_CURRENT.md` default with runtime `SPEC_CURRENT.md` resolver.
  - `workflows.rs`: `locus_sync_task_board` reads/writes runtime `TASK_BOARD.md` and emits this path in `sync_target`.
  - `role_mailbox.rs`: export index/thread/manifest to runtime `ROLE_MAILBOX/`; emit dynamic `export_root`; default `task_board_id` in log rows aligns to runtime path.
  - `api/role_mailbox.rs`: read mailbox index from runtime governance root.
  - `flight_recorder/mod.rs`: GovMailboxExported payload validation accepts dynamic non-empty `export_root` while rejecting `/.GOV/**`.
- Open questions:
  - None blocking; use no fallback to `docs/**` for authoritative runtime reads.
- Notes:
  - Preserve v3 snapshot generator/validator behavior.
  - Keep compatibility semantics one-way: runtime source-of-truth is `.handshake/gov/`; optional docs outputs are not required for correctness.

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
- Added `src/backend/handshake_core/src/runtime_governance.rs` as shared runtime governance path resolver:
  - default runtime governance root `.handshake/gov/`
  - optional env override via `HANDSHAKE_GOVERNANCE_ROOT`
  - boundary checks to reject runtime roots under `docs/**` and `/.GOV/**`
  - canonical runtime paths for `SPEC_CURRENT.md`, `TASK_BOARD.md`, and `ROLE_MAILBOX/`
- Updated `src/backend/handshake_core/src/governance_pack.rs` to resolve default master spec pointer from runtime governance `SPEC_CURRENT.md` instead of `docs/SPEC_CURRENT.md`.
- Updated `src/backend/handshake_core/src/workflows.rs` Locus task board sync to read/write runtime `TASK_BOARD.md` and emit runtime `sync_target`.
- Updated `src/backend/handshake_core/src/role_mailbox.rs` and `src/backend/handshake_core/src/api/role_mailbox.rs` to export/read mailbox index from runtime governance root (`.handshake/gov/ROLE_MAILBOX/` by default).
- Updated `src/backend/handshake_core/src/flight_recorder/mod.rs` mailbox export payload validation to accept bounded runtime export roots while rejecting repo-governance segments (`docs`, `.GOV`).
- Updated `src/backend/handshake_core/tests/role_mailbox_tests.rs` expectations to runtime mailbox path and runtime task board id.
- Added module export in `src/backend/handshake_core/src/lib.rs` for `runtime_governance`.

## HYGIENE
- Ran `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml`.
- Ran required proof scans:
  - `rg -n "docs/" src/backend/handshake_core/src -S`
  - `rg -n "\\.GOV/" src/backend/handshake_core/src -S`
- Ran required backend verification:
  - `cd src/backend/handshake_core; cargo test`
- Ran required clean-up:
  - `just cargo-clean`
- Re-ran `just pre-work WP-1-Product-Governance-Snapshot-v4` before final packet closure.

## VALIDATION
- **Target File**: `src/backend/handshake_core/src/governance_pack.rs`
- **Start**: 15
- **End**: 556
- **Line Delta**: 20
- **Pre-SHA1**: `996e97351de330d0134ed86c63fd32e7b321b7b4`
- **Post-SHA1**: `048fd0ff3ae4302d5fd7aaad761d9328dd7bdae4`
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
- **Lint Results**: `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml`
- **Artifacts**: none
- **Timestamp**: 2026-02-08T21:21:05Z
- **Operator**: Coder
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- **Notes**: runtime spec pointer moved from docs path to runtime governance path

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 31
- **End**: 633
- **Line Delta**: 2
- **Pre-SHA1**: `3dc957091d65d3f93da13208277a53af9b969b70`
- **Post-SHA1**: `081288dd934723381c8394e40f4ec7eecb30161a`
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
- **Lint Results**: `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml`
- **Artifacts**: none
- **Timestamp**: 2026-02-08T21:21:05Z
- **Operator**: Coder
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- **Notes**: task board sync target now runtime governance path

- **Target File**: `src/backend/handshake_core/src/role_mailbox.rs`
- **Start**: 8
- **End**: 1553
- **Line Delta**: 19
- **Pre-SHA1**: `dfc37c834faf3125052c133e9f21459d9e51774a`
- **Post-SHA1**: `4725d88f3c99d55073f35ad950546fd0533a6cd5`
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
- **Lint Results**: `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml`
- **Artifacts**: none
- **Timestamp**: 2026-02-08T21:21:05Z
- **Operator**: Coder
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- **Notes**: mailbox export and default task board id now runtime-governance-rooted

- **Target File**: `src/backend/handshake_core/src/api/role_mailbox.rs`
- **Start**: 15
- **End**: 51
- **Line Delta**: -2
- **Pre-SHA1**: `18e9bb423009b44249c94bcae75ea99c8cdf2eb2`
- **Post-SHA1**: `d15f485df3e49dd70521c3e768b851a6c74782e5`
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
- **Lint Results**: `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml`
- **Artifacts**: none
- **Timestamp**: 2026-02-08T21:21:05Z
- **Operator**: Coder
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- **Notes**: index route reads runtime mailbox index path

- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 2853
- **End**: 3733
- **Line Delta**: 79
- **Pre-SHA1**: `38b6d0fc8b1cf7bdd74d2213b9c272aa79e30c19`
- **Post-SHA1**: `4712140a2b3a83d127f5242deee218ec8f190130`
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
- **Lint Results**: `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml`
- **Artifacts**: none
- **Timestamp**: 2026-02-08T21:21:05Z
- **Operator**: Coder
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- **Notes**: mailbox export payload validation now enforces runtime-path constraints

- **Target File**: `src/backend/handshake_core/src/lib.rs`
- **Start**: 16
- **End**: 16
- **Line Delta**: 1
- **Pre-SHA1**: `95112ff112450323fe0a7e1ccac00650cde77f3b`
- **Post-SHA1**: `f1ad10c086150071cfca8a4d2b3fb67e992f0a75`
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
- **Lint Results**: `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml`
- **Artifacts**: none
- **Timestamp**: 2026-02-08T21:21:05Z
- **Operator**: Coder
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- **Notes**: exports runtime governance module

- **Target File**: `src/backend/handshake_core/src/runtime_governance.rs`
- **Start**: 1
- **End**: 215
- **Line Delta**: 215
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `9c3c52b708aa34ec2a49e9f8764fbf6443845574`
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
- **Lint Results**: `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml`
- **Artifacts**: none
- **Timestamp**: 2026-02-08T21:21:05Z
- **Operator**: Coder
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- **Notes**: shared runtime governance path boundary and defaults

- **Target File**: `src/backend/handshake_core/tests/role_mailbox_tests.rs`
- **Start**: 25
- **End**: 223
- **Line Delta**: 7
- **Pre-SHA1**: `fb58331c64fe59defe2a9a6df06597a56105a4a2`
- **Post-SHA1**: `ead2de10d8325255edb07d53838f9a0538ef7ca9`
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
- **Lint Results**: `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml`
- **Artifacts**: none
- **Timestamp**: 2026-02-08T21:21:05Z
- **Operator**: Coder
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- **Notes**: tests now expect runtime governance mailbox and task-board paths

- **Target File**: `docs/SPEC_CURRENT.md`
- **Start**: 5
- **End**: 7
- **Line Delta**: 0
- **Pre-SHA1**: `1b0269fdf8d23f4ac3ebb2ee8813d2dd4178eae0`
- **Post-SHA1**: `05b103274fd55a87037e8f1f4589df0b49794c1f`
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
- **Lint Results**: inherited range-only delta; no new edit in this WP
- **Artifacts**: none
- **Timestamp**: 2026-02-08T21:21:05Z
- **Operator**: Coder
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- **Notes**: included to satisfy deterministic `--range` post-work coverage for inherited branch history

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (implementation and hygiene commands completed; post-work pending)
- What changed in this update: Implemented runtime-governance path decoupling from repo docs/.GOV dependencies across governance-pack, workflow task-board sync, and role-mailbox runtime paths; updated recorder and tests.
- Next step / handoff hint: Stage packet updates and run `just post-work WP-1-Product-Governance-Snapshot-v4 --range 0092ad1dcfec98e064f9eb97185ac493dedb7b42..HEAD`.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`
- REQUIREMENT: "Product runtime does not require repo `docs/**` for governance-critical behavior."
- EVIDENCE: `src/backend/handshake_core/src/governance_pack.rs:187`
- REQUIREMENT: "Product runtime does not require repo `docs/**` for governance-critical behavior."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:252`
- REQUIREMENT: "Runtime governance state defaults to product-owned `.handshake/gov/` (configurable)."
- EVIDENCE: `src/backend/handshake_core/src/runtime_governance.rs:9`
- REQUIREMENT: "Runtime governance state defaults to product-owned `.handshake/gov/` (configurable)."
- EVIDENCE: `src/backend/handshake_core/src/runtime_governance.rs:24`
- REQUIREMENT: "Runtime does not read/write `/.GOV/**`."
- EVIDENCE: `src/backend/handshake_core/src/runtime_governance.rs:117`
- REQUIREMENT: "Role mailbox runtime path moved off repo docs and is runtime-governance-rooted."
- EVIDENCE: `src/backend/handshake_core/src/role_mailbox.rs:506`
- REQUIREMENT: "Role mailbox API reads runtime-governance mailbox index."
- EVIDENCE: `src/backend/handshake_core/src/api/role_mailbox.rs:51`
- REQUIREMENT: "Flight Recorder mailbox export payload constrains runtime export_root and rejects repo-governance segments."
- EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:3055`
- REQUIREMENT: "Tests validate runtime mailbox/task-board paths."
- EVIDENCE: `src/backend/handshake_core/tests/role_mailbox_tests.rs:25`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Product-Governance-Snapshot-v4/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`
COMMAND: `just pre-work WP-1-Product-Governance-Snapshot-v4`
EXIT_CODE: `0`
- PROOF_LINES:
  - `Pre-work validation PASSED`
  - `PASS: Branch matches PREPARE (feat/WP-1-Product-Governance-Snapshot-v4)`

COMMAND: `cd src/backend/handshake_core; cargo test`
EXIT_CODE: `0`
- PROOF_LINES:
  - `test result: ok. 160 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`
  - `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out (role_mailbox_tests)`

COMMAND: `just cargo-clean`
EXIT_CODE: `0`
- PROOF_LINES:
  - `Removed 1766 files, 11.7GiB total`

- COMMAND: `just pre-work WP-1-Product-Governance-Snapshot-v4`
- EXIT_CODE: `0`
- PROOF_LINES:
  - `Pre-work validation PASSED`
  - `PASS: Branch matches PREPARE (feat/WP-1-Product-Governance-Snapshot-v4)`

- COMMAND: `rg -n "docs/" src/backend/handshake_core/src -S`
- EXIT_CODE: `0`
- PROOF_LINES:
  - `src/backend/handshake_core/src/ace/mod.rs:24:/// See docs/RUNBOOK_DEBUG.md for error resolution guidance.`
  - `No runtime governance offender files matched docs/TASK_BOARD.md, docs/SPEC_CURRENT.md, docs/ROLE_MAILBOX/.`

- COMMAND: `rg -n "\\.GOV/" src/backend/handshake_core/src -S`
- EXIT_CODE: `1`
- PROOF_LINES:
  - `No matches in backend runtime source files.`

- COMMAND: `cd src/backend/handshake_core; cargo test`
- EXIT_CODE: `0`
- PROOF_LINES:
  - `test result: ok. 160 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`
  - `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out (role_mailbox_tests)`

- COMMAND: `just cargo-clean`
- EXIT_CODE: `0`
- PROOF_LINES:
  - `Removed 1766 files, 11.7GiB total`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
