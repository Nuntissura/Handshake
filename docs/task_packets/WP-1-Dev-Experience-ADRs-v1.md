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
- **Status:** Done
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
  - docs/adr/ADR-0002-runtime-selection-ollama.md
  - docs/adr/ADR-0003-db-layout-jobs-and-flight-recorder.md
  - docs/adr/ADR-0004-capability-model-shape.md
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
- Updated `docs/START_HERE.md` to replace Phase 1 runtime TBD with concrete Ollama install/run/verify steps and env overrides.
- Updated `justfile` so `just dev` runs an Ollama preflight check (GET `${OLLAMA_URL}/api/tags`) and fails fast with a clear message when unreachable.
- Added ADRs:
  - `docs/adr/ADR-0002-runtime-selection-ollama.md`
  - `docs/adr/ADR-0003-db-layout-jobs-and-flight-recorder.md`
  - `docs/adr/ADR-0004-capability-model-shape.md`

## HYGIENE
- Commands run (see chat logs for verbatim output):
  - `just validator-scan`
  - `just validator-dal-audit`
  - `just validator-git-hygiene`
  - `just cargo-clean`
  - `just post-work WP-1-Dev-Experience-ADRs-v1`

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `justfile`
- **Start**: 1
- **End**: 30
- **Line Delta**: 0
- **Pre-SHA1**: `403b73df8d06db47a2ccb99965f4344f6db6135b`
- **Post-SHA1**: `3011f90a1a4bd38a36f158f4d532ee94c1622bc9`
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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.115.md
- **Notes**: Added Ollama preflight to `just dev` and documented Phase 1 Ollama setup; created initial ADRs per scope.

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Ready for validator review
- What changed in this update:
  - `docs/START_HERE.md` - Phase 1 Ollama setup steps + env overrides (replaces HSK-1001 TBD)
  - `justfile` - add `preflight-ollama` and run it before `dev`
  - `docs/adr/ADR-0002-runtime-selection-ollama.md` (new)
  - `docs/adr/ADR-0003-db-layout-jobs-and-flight-recorder.md` (new)
  - `docs/adr/ADR-0004-capability-model-shape.md` (new)
- Next step / handoff hint:
  - Validator: review commits `0ce4bb7e` (bootstrap claim) and `34b089df` (implementation), then proceed per VALIDATOR_PROTOCOL.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

### 2026-01-24 - Remediation Evidence (Coder)
Command: `just preflight-ollama`
Output:
```text
node -e "const base=(process.env.OLLAMA_URL||'http://localhost:11434'); const normalized=base.endsWith('/')?base.slice(0,-1):base; const url=normalized + '/api/tags'; const lib=url.startsWith('https://')?require('https'):require('http'); const req=lib.get(url,(res)=>{ const ok=!!res.statusCode && res.statusCode>=200 && res.statusCode<300; if(ok){ process.exit(0); } console.error('Ollama preflight failed: GET ' + url + ' returned ' + res.statusCode + '. Install Ollama (Windows: winget install -e --id Ollama.Ollama), then run ollama serve (or ollama run mistral), or set OLLAMA_URL.'); process.exit(1); }); req.on('error',()=>{ console.error('Ollama preflight failed: cannot reach ' + url + '. Install Ollama (Windows: winget install -e --id Ollama.Ollama), then run ollama serve (or ollama run mistral), or set OLLAMA_URL.'); process.exit(1); }); req.setTimeout(3000, ()=>req.destroy(new Error('timeout')));"
Ollama preflight failed: cannot reach http://localhost:11434/api/tags. Install Ollama (Windows: winget install -e --id Ollama.Ollama), then run ollama serve (or ollama run mistral), or set OLLAMA_URL.
error: Recipe `preflight-ollama` failed on line 11 with exit code 1
```

Command: `just post-work WP-1-Dev-Experience-ADRs-v1`
Output:
```text
Checking Phase Gate for WP-1-Dev-Experience-ADRs-v1...
? GATE PASS: Workflow sequence verified.

Post-work validation for WP-1-Dev-Experience-ADRs-v1 (deterministic manifest + gates)...

Check 1: Validation manifest present

Check 2: Manifest fields

Check 3: File integrity (per manifest entry)

Check 4: Git status

==================================================
Post-work validation PASSED

You may proceed with commit.
? ROLE_MAILBOX_EXPORT_GATE PASS
```

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

---

### 2026-01-24 - VALIDATION REPORT - WP-1-Dev-Experience-ADRs-v1 (RE-APPENDED)
Verdict: FAIL

Scope Inputs:
- Task Packet: `docs/task_packets/WP-1-Dev-Experience-ADRs-v1.md` (status: In Progress)
- Spec: `docs/SPEC_CURRENT.md` -> `Handshake_Master_Spec_v02.115.md` (anchors per packet: 4.2.2.2; 2.6.6.6.3)

Files Checked:
- `docs/task_packets/WP-1-Dev-Experience-ADRs-v1.md`
- `docs/START_HERE.md`
- `justfile`
- `docs/adr/ADR-0002-runtime-selection-ollama.md`
- `docs/adr/ADR-0003-db-layout-jobs-and-flight-recorder.md`
- `docs/adr/ADR-0004-capability-model-shape.md`

Findings:
- PASS (Docs): `docs/START_HERE.md:95` documents Phase 1 Ollama setup; env overrides at `docs/START_HERE.md:104-105`.
- PASS (ADRs): ADRs exist at `docs/adr/ADR-0002-runtime-selection-ollama.md:1`, `docs/adr/ADR-0003-db-layout-jobs-and-flight-recorder.md:1`, `docs/adr/ADR-0004-capability-model-shape.md:1`.
- FAIL (Dev preflight): `justfile:6` wires `dev: preflight-ollama`, but `justfile:10` `preflight-ollama` fails with a Node syntax error instead of a clear actionable "Ollama unreachable" message.
  - Evidence (verbatim excerpt):
    ```text
    > just preflight-ollama
    SyntaxError: Invalid regular expression flags
    error: Recipe `preflight-ollama` failed ...
    ```
- NOTE (Deterministic gate): `just post-work WP-1-Dev-Experience-ADRs-v1` fails on a clean tree ("No files changed (git status clean)") by design; a PASS must be recorded pre-commit in `## EVIDENCE`.

Tests / Commands Run:
- `just pre-work WP-1-Dev-Experience-ADRs-v1`: PASS
- `just preflight-ollama`: FAIL
- `just post-work WP-1-Dev-Experience-ADRs-v1`: FAIL (clean tree)

REASON FOR FAIL:
- WP DONE_MEANS requires `just dev` (and/or a dedicated preflight target) to fail fast with a clear message when Ollama is missing/unreachable at `OLLAMA_URL`. Current `preflight-ollama` fails with a Node parse error, blocking dev with a misleading failure.

Required Remediation (Coder):
1. Fix `justfile` `preflight-ollama` to work under the repo's PowerShell `windows-shell` and to:
   - exit 0 when Ollama is reachable
   - exit nonzero with a clear actionable message when unreachable
2. Paste verbatim outputs into `## EVIDENCE`:
   - `just preflight-ollama`
   - `just post-work WP-1-Dev-Experience-ADRs-v1` (run pre-commit)
3. Commit remediation and request re-validation.

Note:
- This report is re-appended because the prior appended block was lost due to later packet edits. Append-only validation history is mandatory.

---

### 2026-01-24 - VALIDATION REPORT - WP-1-Dev-Experience-ADRs-v1 (REVALIDATION)
Verdict: PASS

Scope Inputs:
- Task Packet: `docs/task_packets/WP-1-Dev-Experience-ADRs-v1.md` (status: In Progress)
- Spec: `docs/SPEC_CURRENT.md` -> `Handshake_Master_Spec_v02.115.md` (anchors per packet: 4.2.2.2; 2.6.6.6.3)

Files Checked:
- `docs/task_packets/WP-1-Dev-Experience-ADRs-v1.md`
- `docs/refinements/WP-1-Dev-Experience-ADRs-v1.md`
- `docs/START_HERE.md`
- `justfile`
- `docs/adr/ADR-0002-runtime-selection-ollama.md`
- `docs/adr/ADR-0003-db-layout-jobs-and-flight-recorder.md`
- `docs/adr/ADR-0004-capability-model-shape.md`

Findings:
- PASS (Docs prerequisite): `docs/START_HERE.md:95` provides explicit Windows Ollama install/run/verify steps; env overrides documented at `docs/START_HERE.md:104`.
- PASS (Fail-fast dev): `justfile:6` wires `dev: preflight-ollama`; `justfile:10` defines `preflight-ollama` with clear actionable error output when Ollama is unreachable.
- PASS (ADRs): `docs/adr/ADR-0002-runtime-selection-ollama.md:1`, `docs/adr/ADR-0003-db-layout-jobs-and-flight-recorder.md:1`, `docs/adr/ADR-0004-capability-model-shape.md:1`.
- PASS (No model weights committed): `git ls-files | rg "\\.(gguf|safetensors|pt|pth)$"` returned no matches during validation.
- Forbidden Pattern Audit (in-scope): grep for `unwrap/expect/todo/panic/serde_json::Value/placeholder` showed only doc text describing directory placeholders (`docs/START_HERE.md:53-54`); no production-path placeholder/stub patterns introduced by this WP.

Evidence:
- `just preflight-ollama` behavior (actionable failure when Ollama is unreachable) is recorded verbatim in `## EVIDENCE` (Remediation Evidence block).
- `just post-work WP-1-Dev-Experience-ADRs-v1` PASS output is recorded verbatim in `## EVIDENCE` (Remediation Evidence block).

REASON FOR PASS:
- WP DONE_MEANS are satisfied: Phase 1 Ollama setup is explicitly documented, `just dev` fails fast via a working `preflight-ollama` with actionable error output, required ADRs exist, and there is no evidence of model-weight files being committed.
