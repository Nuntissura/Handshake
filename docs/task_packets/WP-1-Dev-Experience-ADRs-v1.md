# Task Packet: WP-1-Dev-Experience-ADRs-v1

## METADATA
- TASK_ID: WP-1-Dev-Experience-ADRs-v1
- WP_ID: WP-1-Dev-Experience-ADRs-v1
- BASE_WP_ID: WP-1-Dev-Experience-ADRs
- DATE: 2026-01-23T22:28:15.431Z
- REQUESTOR: User
- AGENT_ID: codex-cli
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja230120262310

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Dev-Experience-ADRs-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Make Phase 1 developer startup functional and deterministic by requiring a local model runtime (Ollama) and documenting the concrete install/run/config path; create initial ADRs for key architectural choices to prevent drift.
- Why: The Operator machine currently lacks Ollama, blocking Phase 1 LLM-backed functionality; without explicit docs + ADRs, dev onboarding and decision provenance drift quickly.
- IN_SCOPE_PATHS:
  - docs/START_HERE.md
  - justfile
  - docs/adr/ADR-0002-runtime-selection-ollama.md (new)
  - docs/adr/ADR-0003-db-layout-jobs-and-flight-recorder.md (new)
  - docs/adr/ADR-0004-capability-model-shape.md (new)
- OUT_OF_SCOPE:
  - Committing any model weights (`*.gguf`, etc.) or other large artifacts to git.
  - Cross-platform installer work beyond Windows (Phase 1 machine is Windows).
  - Re-architecting backend LLM routing/providers beyond wiring/config needed for Ollama dev flow.
  - Any Flight Recorder schema changes (this WP adds docs/ADRs; schema work belongs in schema/FR packets).

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Dev-Experience-ADRs-v1

# Manual (Operator terminal; do NOT run as a blocking tool):
# - Install Ollama (Windows): winget install -e --id Ollama.Ollama
# - Start server: ollama serve
# - Pull+run a small-ish model for smoke: ollama run mistral
# - Verify required env overrides (as needed):
#   - OLLAMA_URL=http://localhost:11434
#   - OLLAMA_MODEL=mistral
# - Start Handshake dev: just dev

just cargo-clean
just post-work WP-1-Dev-Experience-ADRs-v1
```

### DONE_MEANS
- `docs/START_HERE.md` contains explicit Phase 1 setup steps for Ollama on Windows (install, run, verify) and removes any Phase 1-critical TBDs for local model runtime.
- `just dev` (and/or a dedicated preflight target) fails fast with a clear message when Ollama is missing or not reachable at `OLLAMA_URL` (Option B: required for this phase).
- ADRs exist under `docs/adr/` for: runtime selection (Ollama), DB layout for jobs+Flight Recorder, and capability model shape.
- No model weights are added to the repo.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.115.md (recorded_at: 2026-01-23T22:28:15.431Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 4.2.2.2 (Ollama - The Easy Choice); 2.6.6.6.3 (Schema Contracts - schema changes linked to ADRs); [CX-208] ROOT_DOCS_CANONICAL; [CX-209] NAVIGATION_PACK_FILES
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packet artifacts:
  - docs/task_packets/stubs/WP-1-Dev-Experience-ADRs.md (stub; not executable)
- Preserved vs changed:
  - Preserved: Stub intent (one-command dev startup + ADRs) and Phase 1 local model runtime requirement.
  - Changed: Activated as an official packet (v1) under current SPEC_CURRENT, with Option B posture (Ollama required) and explicit in-scope paths.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.115.md
  - docs/adr/ADR-0001-handshake-architecture-and-governance.md
  - justfile
- SEARCH_TERMS:
  - "#### 4.2.2.2 Ollama"
  - "Schema Contracts"
  - "linked to ADRs"
  - "OLLAMA_URL"
  - "OLLAMA_MODEL"
  - "localhost:11434"
  - "just dev"
  - "HSK-1001"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Dev-Experience-ADRs-v1

  # Manual (Operator terminal): winget install -e --id Ollama.Ollama
  # Manual (separate terminal): ollama serve
  # Manual smoke: ollama run mistral

  # Manual (separate terminal): just dev
  just post-work WP-1-Dev-Experience-ADRs-v1
  ```
- RISK_MAP:
  - "Ollama missing/old version" -> "Dev startup blocked; false-negative runtime failures"
  - "Port 11434 conflict" -> "Ollama unreachable; misleading LLM errors"
  - "Model too large for hardware" -> "Timeouts/OOM; poor developer experience"
  - "Accidental model weight commit" -> "Repo bloat; policy violation"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

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
