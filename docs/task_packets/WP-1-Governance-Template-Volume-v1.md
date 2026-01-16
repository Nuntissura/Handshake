# Task Packet: WP-1-Governance-Template-Volume-v1

## METADATA
- TASK_ID: WP-1-Governance-Template-Volume-v1
- WP_ID: WP-1-Governance-Template-Volume-v1
- BASE_WP_ID: WP-1-Governance-Template-Volume (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-16T02:33:10.494Z
- REQUESTOR: Operator (ilja)
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
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

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
