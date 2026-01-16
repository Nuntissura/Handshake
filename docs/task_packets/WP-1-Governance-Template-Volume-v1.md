# Task Packet: WP-1-Governance-Template-Volume-v1

## METADATA
- TASK_ID: WP-1-Governance-Template-Volume-v1
- WP_ID: WP-1-Governance-Template-Volume-v1
- BASE_WP_ID: WP-1-Governance-Template-Volume (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-16T02:33:10.494Z
- REQUESTOR: Operator (ilja)
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja160120260327

## USER_CONTEXT (Non-Technical Explainer) [APPEND-ONLY]
- What you get: a generated, ready-to-use governance folder for a new project (task packet templates, protocols, gates, `just` commands, etc.) exported to a directory you choose.
- Where it comes from: the template text is taken from the current Master Spec "Governance Pack: Template Volume" section, so the spec stays the source-of-truth.
- What gets filled in: placeholders like project name/code and directory layout (frontend/backend root paths) are substituted from a small set of project invariants you provide.
- Safety: the exporter must refuse unsafe paths (e.g., `..` traversal) and must default to NOT overwriting existing non-empty directories unless you explicitly allow it.
- Audit trail: each export run must write a Flight Recorder ExportRecord-style event so exports are traceable and reproducible.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Governance-Template-Volume-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement deterministic export/rendering of the inlined Governance Pack Template Volume (spec 7.5.4.9.3) into a concrete governance repo directory, with all placeholders resolved from project invariants (spec 7.5.4.8/7.5.4.9.1) and with safety constraints (no path traversal; default-deny overwrites).
- Why: Handshake must be able to generate the same strict multi-role governance workflow (codex + protocols + gates + task board + scripts) for arbitrary projects without Handshake-hardcoding, so future projects can reuse this governance/mechanical-gates system.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/api/mod.rs
  - src/backend/handshake_core/src/api/governance_pack.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/governance_pack.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/workflows.rs
  - app/src/App.tsx
  - app/src/components/operator/GovernancePackExport.tsx
  - app/src/components/operator/index.ts
  - app/src/lib/api.ts
- OUT_OF_SCOPE:
  - Editing `Handshake_Master_Spec_v02.112.md` Template Volume bodies (source-of-truth is the spec; this WP implements the exporter only)
  - Editing `Handshake Codex v1.4.md`, `AGENTS.md`, or any role protocol files in this repo (exporter consumes canonical templates; does not change them)
  - Implementing the full multi-model governance runtime beyond export/materialize (separate WPs)

## HARDENED_INVARIANTS (RISK_TIER=HIGH)
- Path safety: refuse absolute paths and any `..` segments in template relpaths; enforce export-root confinement (no writes outside the selected directory).
- Overwrite safety: default-deny overwriting existing non-empty export dirs; require explicit operator confirmation/flag for overwrite mode.
- Leakage safety: never log/export secrets; avoid emitting absolute filesystem paths unless explicitly required by ExportRecord semantics; prefer relative paths in logs/manifests.
- Determinism: stable file write order; deterministic bytes (avoid OS-dependent path separators/line endings); fail if any `{{...}}` placeholder remains unresolved.
- Source-of-truth: exporter MUST read templates from the Master Spec Template Volume markers (7.5.4.9.3), not from the repo working tree, to prevent drift.

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Governance-Template-Volume-v1
# Backend checks:
cd src/backend/handshake_core; cargo fmt
cd src/backend/handshake_core; cargo clippy --all-targets --all-features
cd src/backend/handshake_core; cargo test
# Frontend checks (if UI hook is implemented in this WP):
cd app; pnpm run lint
cd app; pnpm test
just cargo-clean
just post-work WP-1-Governance-Template-Volume-v1
```

### DONE_MEANS
- 7.5.4.9: Exporter extracts templates from the current Master Spec Template Volume markers (`GOV_PACK_TEMPLATE_VOLUME_BEGIN/END`) and writes the full Template Index file set to the chosen export directory, with stable write order.
- 7.5.4.9: Exported files contain no unresolved `{{...}}` placeholders; missing required placeholders fail fast with actionable errors.
- 7.5.4.8: Exporter prompts/accepts project identity + layout invariants (project code/name, naming policy, language_layout_profile_id, tool paths, role_mailbox_export_dir default) and renders templates without any `Handshake_*` hardcoding.
- 2.3.10.1-2.3.10.3: Export run emits an ExportRecord-equivalent Flight Recorder event (includes export_id, determinism_level, and LocalFile materialized path(s)).
- Operator UX: In Handshake UI, Operator can trigger Governance Pack export and is prompted to pick the target directory; UI surfaces the resulting job/export ID or error message.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.112.md (recorded_at: 2026-01-16T02:33:10.494Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- ORCHESTRATOR_AUTHORITY_DOCS:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - docs/RUNBOOK_DEBUG.md
  - docs/QUALITY_GATE.md
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.112.md 7.5.4.8 (Governance Pack: Project-Specific Instantiation) (HARD)
  - Handshake_Master_Spec_v02.112.md 7.5.4.9 (Governance Pack: Template Volume) (HARD)
  - Handshake_Master_Spec_v02.112.md 7.5.4.9.3 (Template Bodies markers) (HARD)
  - Handshake_Master_Spec_v02.112.md 2.3.10.1-2.3.10.3 (Export pipeline + ExportRecord + determinism) (Normative)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packets:
  - docs/task_packets/stubs/WP-1-Governance-Template-Volume-v1.md (status: STUB; non-executable planning placeholder)
- This packet is the initial activation (`-v1`) for `WP-1-Governance-Template-Volume` and preserves the stub intent (spec-template parsing + placeholder resolution + deterministic export) while adding signed Technical Refinement anchors and a concrete in-scope file list.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.112.md
  - src/backend/handshake_core/src/governance_pack.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/api/governance_pack.rs
  - src/backend/handshake_core/src/api/bundles.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - app/src/components/operator/GovernancePackExport.tsx
  - app/src/lib/api.ts
- SEARCH_TERMS:
  - "GOV_PACK_TEMPLATE_VOLUME_BEGIN"
  - "GOV_PACK_TEMPLATE_VOLUME_END"
  - "Template File:"
  - "Placeholder Glossary"
  - "{{PROJECT_CODE}}"
  - "docs/PROJECT_INVARIANTS.md"
  - "JobKind::DebugBundleExport"
  - "FlightRecorderEventType::DebugBundleExport"
  - "Write an ExportRecord"
  - "materialized_paths"
  - "determinism_level"
  - "export_target"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Governance-Template-Volume-v1
  just gate-check WP-1-Governance-Template-Volume-v1
  cd src/backend/handshake_core; cargo test
  cd app; pnpm test
  ```
- RISK_MAP:
  - "template parsing drift" -> "wrong exported file set; breaks kernel parity"
  - "placeholder resolution incomplete" -> "exported repo contains unresolved {{...}}; unusable governance pack"
  - "path traversal / unsafe writes" -> "writes outside export root; potential data loss/security issue"
  - "non-deterministic ordering/newlines" -> "hash drift; conformance failure across OSes"
  - "ExportRecord/event missing fields" -> "violates 2.3.10 auditability requirements"

## SKELETON
- Proposed interfaces/types/contracts:
  - `GovernancePackTemplate { rel_path: String, body: String }`
  - `extract_template_volume(spec_text: &str) -> Vec<GovernancePackTemplate>` (parses 7.5.4.9.3 markers + per-template code fences)
  - `PlaceholderResolver` that fills the 7.5.4.9.1 glossary placeholders from an explicit `ProjectIdentity`
  - `export_governance_pack(request) -> ExportResult` (writes templates with safety + determinism; emits Flight Recorder ExportRecord event)
- Open questions:
  - Should the export create a directory artifact + then materialize (2.3.10.6) or treat the directory as materialized-only for v1?
  - Do we emit a ZIP bundle variant in addition to LocalFile directory materialization (2.3.10.7), or defer to a later WP?
- Notes:
  - Source-of-truth for exported templates is the current Master Spec Template Volume block (do not export from the repo working tree to avoid drift).

## IMPLEMENTATION
- Backend:
  - Added `governance_pack` module that extracts the Template Volume from the current Master Spec (between `GOV_PACK_TEMPLATE_VOLUME_BEGIN/END`), parses `###### Template File:` headers + ```` fences, and writes rendered templates in deterministic order.
  - Placeholder policy: scans for all `{{TOKEN}}` occurrences in Template Volume, requires coverage for every token, and hard-errors with `{token, template_file}`; also fails if any placeholder remains post-render.
  - Path/materialize policy: `ExportTarget::LocalFile { path: PathBuf }` (absolute) with traversal-safe confinement via canonicalized export root + canonicalized parent dirs; atomic file writes (temp + fsync + rename).
  - Export audit: emits `governance_pack_export` Flight Recorder event with ExportRecord-shaped payload meeting Spec 2.3.10.2 normative minimum; `materialized_paths[]` is root-relative + normalized + sorted.
- Jobs/API:
  - Added `JobKind::GovernancePackExport` + capability SSoT mapping (`export.governance_pack`) and a server-enforced profile for the job kind.
  - Added `POST /api/governance_pack/export` that queues `governance_pack_export` jobs and starts workflow execution.
- Frontend:
  - Added operator modal `GovernancePackExport` to collect export directory + invariants and poll job status.

## HYGIENE
- TEST_PLAN run (exit codes recorded; no manual verdicts):
  - `just pre-work WP-1-Governance-Template-Volume-v1` (exit 0)
  - `cd src/backend/handshake_core; cargo fmt` (exit 0)
  - `cd src/backend/handshake_core; cargo clippy --all-targets --all-features` (exit 0; clippy warning: `clippy::too_many_arguments` in `src/backend/handshake_core/src/role_mailbox.rs`)
  - `cd src/backend/handshake_core; cargo test` (exit 0)
  - `cd app; pnpm run lint` (exit 0)
  - `cd app; pnpm test` (exit 0)
  - `just cargo-clean` (exit 0; ran twice during remediation)
  - `just post-work WP-1-Governance-Template-Volume-v1` (exit 0; warnings: HEAD not available for new files)

## VALIDATION
- (Mechanical manifest for audit; values captured from staged files via `just cor701-sha`. This is not an official validation verdict.)
- **Target File**: `app/src/App.tsx`
- **Start**: 1
- **End**: 190
- **Line Delta**: 9
- **Pre-SHA1**: `82e65f87f74b8a95c1e6619d2221b3badd7a5cbb`
- **Post-SHA1**: `50c415c179aae23dfcca113539a1d0147972cc45`
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

- **Target File**: `app/src/components/operator/GovernancePackExport.tsx`
- **Start**: 1
- **End**: 327
- **Line Delta**: 327
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `078b92fb805217b40fa365ee2eab05a90a34aba5`
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

- **Target File**: `app/src/components/operator/index.ts`
- **Start**: 1
- **End**: 9
- **Line Delta**: 1
- **Pre-SHA1**: `97cb741c3889c10d6ba267f1926ca4a1e8ae52e4`
- **Post-SHA1**: `5580c1547e919a234e8c1b89b980fa148f22b572`
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

- **Target File**: `app/src/lib/api.ts`
- **Start**: 1
- **End**: 605
- **Line Delta**: 39
- **Pre-SHA1**: `d83e63ea14721b7013620dfe0350b3370db9134d`
- **Post-SHA1**: `7eb2fc9abbab5b991dbd159045dbf09896be2ea8`
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

- **Target File**: `src/backend/handshake_core/src/api/governance_pack.rs`
- **Start**: 1
- **End**: 61
- **Line Delta**: 61
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `f2ee0030f236da58db7d991fb741a676fedb9ba0`
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

- **Target File**: `src/backend/handshake_core/src/api/mod.rs`
- **Start**: 1
- **End**: 38
- **Line Delta**: 3
- **Pre-SHA1**: `68de38634a659ca0f4ccbb51b0563e2da8d117be`
- **Post-SHA1**: `47997872f7716ef6ad03601ecac4cf91d851f3da`
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

- **Target File**: `src/backend/handshake_core/src/capabilities.rs`
- **Start**: 1
- **End**: 424
- **Line Delta**: 24
- **Pre-SHA1**: `a2b86ba4a6baf679113876e45272b4932331d544`
- **Post-SHA1**: `50ce284d3463aff3b2b3f80042038ef2cb3755b6`
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

- **Target File**: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- **Start**: 1
- **End**: 1076
- **Line Delta**: 1
- **Pre-SHA1**: `1a2d8278f3c5313465a77797e26bf61421180d0a`
- **Post-SHA1**: `41b29b4b24497715dd003bbc9c6698c7024a2e3a`
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

- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 1
- **End**: 1439
- **Line Delta**: 193
- **Pre-SHA1**: `3a68719fc0b81befe6dbf67a32821c567c9da26c`
- **Post-SHA1**: `984409ff277bde04f63782235703b15407627ed8`
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
- **End**: 960
- **Line Delta**: 960
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `318e0d64dd24261f788bd41cfe489ca27c9c69d6`
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

- **Target File**: `src/backend/handshake_core/src/lib.rs`
- **Start**: 1
- **End**: 33
- **Line Delta**: 1
- **Pre-SHA1**: `38ec385ac8ea343823b713c9f481c1bc3b6a6d53`
- **Post-SHA1**: `e4fddce7b3897b10eec75f382f0035d7a04b5c56`
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
- **End**: 881
- **Line Delta**: 4
- **Pre-SHA1**: `4ea008e4be730428b80af37fda36381ed1138183`
- **Post-SHA1**: `1e17697dd2d2f5935e645cb7323853c4ed24a630`
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
- **End**: 1684
- **Line Delta**: 37
- **Pre-SHA1**: `9156a2645aee05fc819a3103eb63c974ce927415`
- **Post-SHA1**: `c521e4dc8878f7fa8f51b56b47ba2290b7c290e1`
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

- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.112.md

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Implementation complete; ready for Validator audit.
- What changed in this update: Implemented Governance Pack Template Volume export job + API + UI + Flight Recorder audit event.
- Next step / handoff hint: Commit staged changes and send the commit SHA + `feat/WP-1-Governance-Template-Volume-v1` + `D:\\Projects\\LLM projects\\wt-WP-1-Governance-Template-Volume-v1` to Operator/Validator.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
- VALIDATION REPORT - WP-1-Governance-Template-Volume-v1
  - Verdict: FAIL
  - Scope Inputs:
    - Task Packet: `docs/task_packets/WP-1-Governance-Template-Volume-v1.md` (**Status:** In Progress)
    - Spec Target (resolved from `docs/SPEC_CURRENT.md`): `Handshake_Master_Spec_v02.112.md`
    - Worktree/Branch validated: `D:\Projects\LLM projects\wt-WP-1-Governance-Template-Volume-v1` (`feat/WP-1-Governance-Template-Volume-v1`)
  - Files Checked:
    - `docs/SPEC_CURRENT.md`
    - `docs/TASK_BOARD.md`
    - `docs/task_packets/WP-1-Governance-Template-Volume-v1.md`
    - `Handshake_Master_Spec_v02.112.md` (Template Volume markers)
    - `src/backend/handshake_core/src/governance_pack.rs`
    - Forbidden-pattern scan scope: `src/backend/handshake_core/src/**`, `app/src/**`
  - Findings (FAIL Blockers):
    - Git hygiene gate: FAIL (dirty worktree; uncommitted changes present per `git status -sb` in the WP worktree).
    - Forbidden Pattern Audit [CX-573E]: FAIL (production-path `expect`/`unwrap` introduced in exporter code with no waiver recorded):
      - `src/backend/handshake_core/src/governance_pack.rs:284` (`Regex::new(...).expect(...)`)
      - `src/backend/handshake_core/src/governance_pack.rs:614` (`Regex::new(...).expect(...)`)
      - `src/backend/handshake_core/src/governance_pack.rs:661` (`Regex::new(...).expect(...)`)
      - `src/backend/handshake_core/src/governance_pack.rs:665` (`cap.get(0).expect(...)`)
      - `src/backend/handshake_core/src/governance_pack.rs:666` (`cap.get(1).expect(...)`)
      - `src/backend/handshake_core/src/governance_pack.rs:685` (`Regex::new(...).expect(...)`)
    - Deterministic manifest / post-work gate: FAIL (packet `## VALIDATION` checkboxes remain unchecked; no recorded passing `just post-work WP-1-Governance-Template-Volume-v1` output).
    - Template fence parsing mismatch vs 4+ backtick contract: FAIL
      - Opening fence accepts 4+ via `starts_with("````")` (`src/backend/handshake_core/src/governance_pack.rs:317`)
      - Closing fence requires exactly `trim() == "````"` (`src/backend/handshake_core/src/governance_pack.rs:329`)
      - This will fail any template bodies that use 5+ backticks.
    - Missing-placeholder evidence attribution: FAIL (scan-phase error reports `template_file` as the spec path, not the concrete template path):
      - `src/backend/handshake_core/src/governance_pack.rs:593` (`template_file: spec_path...`)
  - Tests:
    - Validator did not run tests due to the hard blockers above (dirty tree + forbidden patterns).
    - Coder-reported commands exist under `## HYGIENE`, but outputs + `just post-work` evidence are not captured in `## EVIDENCE`.
  - Commands executed (validator):
    - `git status -sb` (WP worktree)
    - `rg -n "unwrap\\(|expect\\(|todo!\\(|unimplemented!\\(|dbg!\\(|println!\\(|eprintln!\\(|panic!\\(" src/backend/handshake_core/src app/src`
    - `rg -n "GOV_PACK_TEMPLATE_VOLUME_BEGIN" Handshake_Master_Spec_v02.112.md`
    - `rg -n "GOV_PACK_TEMPLATE_VOLUME_END" Handshake_Master_Spec_v02.112.md`
  - Remediation required (handoff to coder):
    - Commit the current implementation changes (clean `git status -sb`).
    - Remove all production-path `expect`/`unwrap` in the new exporter code; replace with typed error propagation (or record an explicit waiver in `## WAIVERS GRANTED` with justification + scope + expiry).
    - Fix template parsing to support 4+ backticks and require matching-length closing fences (and allow optional info string on opening fence).
    - Track `token -> template rel_path` so `MissingPlaceholder { token, template_file }` reports the actual template file containing the missing token.
    - Run full `## QUALITY_GATE` TEST_PLAN including `just cargo-clean` and `just post-work WP-1-Governance-Template-Volume-v1`, and capture outputs under `## EVIDENCE` (complete the deterministic manifest checkboxes).
  - REASON FOR FAIL:
    - The WP is not in a validatable/shippable state: uncommitted changes, forbidden patterns in new production code without waivers, and missing required deterministic-manifest/post-work evidence.
