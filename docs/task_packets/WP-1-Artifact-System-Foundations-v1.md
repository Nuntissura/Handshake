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

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

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
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
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
  - LOG_PATH: `.handshake/logs/WP-1-Artifact-System-Foundations-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
