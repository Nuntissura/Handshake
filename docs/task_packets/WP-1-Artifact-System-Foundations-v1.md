# Task Packet: WP-1-Artifact-System-Foundations-v1

## METADATA
- TASK_ID: WP-1-Artifact-System-Foundations-v1
- WP_ID: WP-1-Artifact-System-Foundations-v1
- BASE_WP_ID: WP-1-Artifact-System-Foundations (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-02T13:12:29.598Z
- MERGE_BASE_SHA: 4ff4952ac6447bdbdd775fcffc18dd9dae2b39a3
- REQUESTOR: Operator (ilja)
- AGENT_ID: user_orchestrator
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja020220261405
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Artifact-System-Foundations-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Close Phase 1 artifact system foundation requirements: canonical artifact on-disk layout + manifests, canonical hashing rules, a single atomic Materialize API, and retention/pinning/GC behavior that is deterministic and auditable.
- Why: Artifacts are the provenance boundary for exports and evidence bundles. Without one artifact-first path and one Materialize implementation, policy bypass, non-deterministic hashes, and silent disk bloat are high-risk and hard to validate.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/governance_pack.rs
  - src/backend/handshake_core/tests/
- OUT_OF_SCOPE:
  - Remote replication / blob-store GC (Phase 2+)
  - New export formats unrelated to artifact store + hashing + retention invariants

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Artifact-System-Foundations-v1
# ...task-specific commands...
just cargo-clean
just post-work WP-1-Artifact-System-Foundations-v1 --range 4ff4952ac6447bdbdd775fcffc18dd9dae2b39a3..HEAD
```

### DONE_MEANS
- Artifact store + manifest: artifact payloads and `artifact.json` are written and read using the on-disk layout in spec (2.3.10.6), with SHA-256 `content_hash` recorded and validated for file and directory artifacts.
- Materialize: all LocalFile "save/export to path" writes go through one shared Materialize implementation that is atomic (temp + fsync + rename) and rejects path traversal and unsafe paths; it does not bypass ExportGuard/CloudLeakageGuard (2.3.10.1, 2.3.11).
- Export auditability: LocalFile materialize populates `ExportRecord.materialized_paths[]` as normalized, root-relative, sorted paths (Flight Recorder schema checks).
- Retention/GC: engine.janitor `prune` produces a deterministic `PruneReport`, never deletes pinned items, writes the report artifact before unlinking, and emits `meta.gc_summary` to Flight Recorder (2.3.11.0-2.3.11.2).
- Bundle hashing: bundle `content_hash` follows canonical BundleIndex hashing rules (2.3.10.7); raw ZIP-byte hashing is only used when bitwise determinism is guaranteed and explicitly recorded.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.123.md (recorded_at: 2026-02-02T13:12:29.598Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 2.3.10.1, 2.3.10.6-2.3.10.8, 2.3.11.0-2.3.11.2 (normative)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior (stub only): docs/task_packets/stubs/WP-1-Artifact-System-Foundations-v1.md (non-executable planning stub)
- This packet (v1): first executable packet; converts stub draft intent into deterministic scope + spec anchors + gates.
- Preserved: single Materialize path, deterministic hashing, retention/pinning invariants, and auditability goals.
- Changed: formalized Main Body anchors; made DONE_MEANS measurable; constrained IN_SCOPE_PATHS to concrete modules; added E2E closure requirements.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.123.md
  - docs/refinements/WP-1-Artifact-System-Foundations-v1.md
  - docs/task_packets/WP-1-Artifact-System-Foundations-v1.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/governance_pack.rs
- SEARCH_TERMS:
  - "Materialize"
  - "artifact.json"
  - "content_hash"
  - "BundleIndex"
  - "RetentionPolicy"
  - "PruneReport"
  - "meta.gc_summary"
  - "ExportRecord"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Artifact-System-Foundations-v1
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- RISK_MAP:
  - "materialize path traversal" -> "arbitrary filesystem write outside workspace"
  - "policy bypass (exportable=false)" -> "data exfiltration via LocalFile/materialize"
  - "non-canonical hashing" -> "non-verifiable evidence bundles / hash drift"
  - "GC deletes pinned" -> "irreversible data loss"
  - "GC fails to run / non-deterministic" -> "disk bloat and non-auditable cleanup"
- OBSERVATIONS (current code reality):
  - `src/backend/handshake_core/src/governance_pack.rs`: has `ExportRecord` and an atomic write helper (temp + fsync + rename) and emits sorted/normalized `materialized_paths[]`, but does not write artifacts to `.handshake/artifacts/...` yet (`output_artifact_handles[].path` is a placeholder).
  - `src/backend/handshake_core/src/bundles/exporter.rs`: writes bundle files via `fs::File::create` (non-atomic) and does not emit `bundle_index.json`; current bundle hashing is not canonical BundleIndex-based.
  - `src/backend/handshake_core/src/workflows.rs`: has a separate `write_bytes_atomic` helper (temp + rename, no fsync) and multiple ad-hoc artifact writes under `data/...`.
  - `src/backend/handshake_core/src/storage/retention.rs`: janitor currently prunes only `ArtifactKind::Result` via `Database::prune_ai_jobs`; `PruneReport` is not materialized as an artifact before deletions; `meta.gc_summary` is emitted.
  - `src/backend/handshake_core/src/storage/postgres.rs`: `prune_ai_jobs` is currently `NotImplemented`.

## SKELETON
SKELETON APPROVED
- Proposed interfaces/types/contracts:
  - `storage::ArtifactManifest` + `storage::ArtifactLayer` (spec 2.3.10.6): read/write `artifact.json` sidecars and validate recorded `content_hash` and `size_bytes`.
  - `storage::ArtifactStore` helper rooted at `<workspace_root>/.handshake/artifacts`:
    - write file artifacts (`payload` file) and directory artifacts (`payload/` dir)
    - compute SHA-256 content hashing for file and directory artifacts
    - structural directory hashing uses a canonical index (sorted paths + per-item hashes + size_bytes)
  - Shared LocalFile Materialize helper:
    - traversal-safe relative paths (no absolute, no `..`, no `:`/backslashes)
    - atomic writes (temp + fsync + rename) with best-effort dir fsync
    - returns `ExportRecord.materialized_paths[]` as normalized, root-relative, sorted paths
  - Bundle hashing (spec 2.3.10.7):
    - emit `bundle_index.json` (sorted paths + per-item content_hash + size_bytes)
    - set bundle content hash to SHA-256 over canonical BundleIndex (structural determinism)
  - Migration strategy:
    - keep `ace::ArtifactHandle { artifact_id, path }` unchanged in Phase 1
    - treat `path` as a root-relative URI to `.handshake/artifacts/<layer>/<artifact_id>/`
- Open questions:
  - Workspace root for `.handshake/` resolution: use repo root via existing `repo_root_from_manifest_dir()` plumbing unless/until a per-workspace root is introduced.
- Notes:
  - Targeted de-duplication: replace ad-hoc atomic write helpers in `workflows.rs` and `governance_pack.rs` with the shared Materialize implementation.

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: server->filesystem (materialize) and server->storage (artifact store + manifests)
- SERVER_SOURCES_OF_TRUTH:
  - ArtifactManifest + storage metadata (artifact_id, layer, classification, exportable, content_hash)
  - ExportRecord fields written by workflow engine (not client-provided)
- REQUIRED_PROVENANCE_FIELDS:
  - artifact_id, layer, kind, content_hash, size_bytes, classification, exportable
  - export_target, materialized_paths[], job_id, source_entity_refs[], source_artifact_refs[]
  - gc_summary counts and PruneReport provenance
- VERIFICATION_PLAN:
  - Flight Recorder schema validates ExportRecord payload invariants (paths normalized/sorted; no '..').
  - Storage layer verifies pinned exclusion and report-before-delete ordering for GC.
- ERROR_TAXONOMY_PLAN:
  - invalid_path (traversal / unsafe path) vs policy_violation (exportable=false) vs io_failure (fsync/rename) vs gc_invariant_violation (pinned deletion attempt)
- UI_GUARDRAILS:
  - If UI triggers LocalFile export/materialize: surface failure class and do not partially write; no bypass path.
- VALIDATOR_ASSERTIONS:
  - Spec anchors satisfied for artifact layout, hashing, materialize atomicity/guards, and retention invariants (2.3.10.1/2.3.10.6-2.3.10.8/2.3.11).
  - No ad-hoc filesystem writes outside Materialize path for LocalFile exports.

## IMPLEMENTATION
- Implemented Phase-1 artifact store primitives (layout + manifest I/O + hash validation + retention invariants) in `src/backend/handshake_core/src/storage/mod.rs`.
- Routed LocalFile exports/materialize through the shared, atomic Materialize implementation (`materialize_local_dir`) in:
  - `src/backend/handshake_core/src/bundles/exporter.rs`
  - `src/backend/handshake_core/src/governance_pack.rs`
- Implemented deterministic/auditable retention behavior (TTL scan + report artifact before deletions + `meta.gc_summary`) in `src/backend/handshake_core/src/storage/retention.rs`.
- Removed ad-hoc workflow atomic write helper in favor of shared atomic writer (`write_file_atomic`) in `src/backend/handshake_core/src/workflows.rs`.

## HYGIENE
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: `0`

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/bundles/exporter.rs`
- **Start**: 1
- **End**: 1937
- **Line Delta**: 577
- **Pre-SHA1**: `9ad03cb88b3924ec7aff30ebfad1ff7ee6ee0844`
- **Post-SHA1**: `2cf29ae41b8f80c154817e055267ee19d2b8381d`
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

- **Target File**: `src/backend/handshake_core/src/governance_pack.rs`
- **Start**: 1
- **End**: 961
- **Line Delta**: -18
- **Pre-SHA1**: `76e50c11fe79e068d24a29b6dee98ff94e39e28f`
- **Post-SHA1**: `996e97351de330d0134ed86c63fd32e7b321b7b4`
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

- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 1
- **End**: 1542
- **Line Delta**: 567
- **Pre-SHA1**: `a93b2e02e73f861bf075e9a9a02f16bfb1a5e792`
- **Post-SHA1**: `8d4536e8d5be6d31c380981ce326e4828b18e9e4`
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

- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 1
- **End**: 2391
- **Line Delta**: 88
- **Pre-SHA1**: `c89b52efcdcb7e2bf8597fea208163079be42f28`
- **Post-SHA1**: `725645ae9c54231873f14f29a5e1f9ea24bf5ba9`
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

- **Target File**: `src/backend/handshake_core/src/storage/retention.rs`
- **Start**: 1
- **End**: 761
- **Line Delta**: 240
- **Pre-SHA1**: `956195399cda999e510033ff107478b3b72cff72`
- **Post-SHA1**: `3a089db581a754e8500385c74bac71b08d482012`
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

- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 1
- **End**: 2649
- **Line Delta**: 1
- **Pre-SHA1**: `58c017cecd8275202c4728d27f4579da50b8db74`
- **Post-SHA1**: `6b4597ac725d3128685ab1b389384ca647503b7c`
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

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1
- **End**: 6601
- **Line Delta**: 18
- **Pre-SHA1**: `adc310811af0e9a86ad9f723aa879324ed005016`
- **Post-SHA1**: `2893f87593559afa3644edfd13b1f69f8d57899b`
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

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: IMPLEMENTATION (code changes uncommitted)
- What changed in this update:
  - Updated artifact store + hashing + materialize APIs and their LocalFile usage paths.
  - Updated janitor prune to write gc_report artifact first and emit `meta.gc_summary` with deleted artifact IDs + reasons.
- Touched files (uncommitted):
  - `docs/task_packets/WP-1-Artifact-System-Foundations-v1.md`
  - `src/backend/handshake_core/src/storage/mod.rs`
  - `src/backend/handshake_core/src/storage/retention.rs`
  - `src/backend/handshake_core/src/storage/sqlite.rs`
  - `src/backend/handshake_core/src/storage/postgres.rs`
  - `src/backend/handshake_core/src/bundles/exporter.rs`
  - `src/backend/handshake_core/src/workflows.rs`
  - `src/backend/handshake_core/src/governance_pack.rs`
- Next step / handoff hint:
  - Fill `## VALIDATION` manifest entries (COR-701 pre/post sha) and run `just post-work WP-1-Artifact-System-Foundations-v1 --range 4ff4952ac6447bdbdd775fcffc18dd9dae2b39a3..HEAD` (paste gate output per protocol).

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`
- REQUIREMENT: "Artifact store + manifest: artifact payloads and `artifact.json` are written and read using the on-disk layout in spec (2.3.10.6), with SHA-256 `content_hash` recorded and validated for file and directory artifacts."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:453`
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:682`
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:715`
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:758`
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:770`
- REQUIREMENT: "Materialize: all LocalFile \"save/export to path\" writes go through one shared Materialize implementation that is atomic (temp + fsync + rename) and rejects path traversal and unsafe paths; it does not bypass ExportGuard/CloudLeakageGuard (2.3.10.1, 2.3.11)."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:523`
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:637`
  - EVIDENCE: `src/backend/handshake_core/src/bundles/exporter.rs:1338`
  - EVIDENCE: `src/backend/handshake_core/src/governance_pack.rs:292`
- REQUIREMENT: "Export auditability: LocalFile materialize populates `ExportRecord.materialized_paths[]` as normalized, root-relative, sorted paths (Flight Recorder schema checks)."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:669`
  - EVIDENCE: `src/backend/handshake_core/src/governance_pack.rs:116`
  - EVIDENCE: `src/backend/handshake_core/src/bundles/exporter.rs:1337`
  - EVIDENCE: `src/backend/handshake_core/src/bundles/exporter.rs:1353`
- REQUIREMENT: "Retention/GC: engine.janitor `prune` produces a deterministic `PruneReport`, never deletes pinned items, writes the report artifact before unlinking, and emits `meta.gc_summary` to Flight Recorder (2.3.11.0-2.3.11.2)."
  - EVIDENCE: `src/backend/handshake_core/src/storage/retention.rs:113`
  - EVIDENCE: `src/backend/handshake_core/src/storage/retention.rs:123`
  - EVIDENCE: `src/backend/handshake_core/src/storage/retention.rs:225`
  - EVIDENCE: `src/backend/handshake_core/src/storage/retention.rs:282`
- REQUIREMENT: "Bundle hashing: bundle `content_hash` follows canonical BundleIndex hashing rules (2.3.10.7); raw ZIP-byte hashing is only used when bitwise determinism is guaranteed and explicitly recorded."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:576`
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:599`
  - EVIDENCE: `src/backend/handshake_core/src/bundles/exporter.rs:47`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - EXIT_CODE: `0`
  - LOG_PATH: `.handshake/logs/WP-1-Artifact-System-Foundations-v1/cargo_test.log` (optional; not committed)

  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Artifact-System-Foundations-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., \"0 failed\")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
