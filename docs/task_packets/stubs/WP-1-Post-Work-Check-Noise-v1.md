# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `docs/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `docs/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Post-Work-Check-Noise-v1

## STUB_METADATA
- WP_ID: WP-1-Post-Work-Check-Noise-v1
- BASE_WP_ID: WP-1-Post-Work-Check-Noise
- CREATED_AT: 2026-02-01T00:50:39Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: docs/SPEC_CURRENT.md
- ROADMAP_POINTER: N/A (governance/tooling UX issue; propose Spec Enrichment later)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.123.md 2.9 Deterministic Edit Process (COR-701) (post-work deterministic manifest gate)
  - Handshake_Master_Spec_v02.123.md 2.7.5 Validation Gates (COR-701 gate surfaces: pre-work/post-work)
  - Handshake_Master_Spec_v02.123.md "scripts/validation/post-work-check.mjs" (mechanical gate implementation reference)
  - docs/ORCHESTRATOR_PROTOCOL.md "Deterministic Manifest & Gate (current workflow, COR-701 discipline)"

## INTENT (DRAFT)
- What: Remove confusing "fatal: path ... exists on disk, but not in '<rev>'" stderr noise emitted during `just post-work` runs (especially in `--range` mode when files are new at the base rev), and make COR-701 hash guidance unambiguous and range-friendly.
- Why: This noise causes humans and agentic roles (Coder/Validator) to misclassify PASS as FAIL, increases babysitting, and slows down WP throughput even when implementation is correct.

## CONTEXT (WHY THIS EXISTS)
Observed during WP validation flows:
- `just post-work WP-... --range <merge-base>..HEAD` can PASS, but still prints one or more lines like:
  - `fatal: path 'X' exists on disk, but not in '<merge-base>'`
- These lines appear when the evaluated diff includes new files (present at HEAD, missing at base). The gate already handles this condition (treats preimage as missing/new-file), but stderr still leaks and looks like a hard failure.

The root mechanical cause:
- Node `child_process.execSync()` without explicit `stdio` settings inherits child stderr in this environment.
- Even if errors are caught, git's stderr output is already emitted to console.
- Minimal reproduction:
  - `node -e "const {execSync}=require('child_process'); try { execSync('git show <base>:<newfile>'); } catch {} console.log('DONE')"`
  - Prints `DONE` and then prints the `fatal: path ... not in '<base>'` line.
  - The fix is to force stderr to pipe (or ignore) for commands that are expected to fail in "new file at base" scenarios:
    - `execSync(cmd, { stdio: ['ignore', 'pipe', 'pipe'] })`

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Make `scripts/validation/post-work-check.mjs` suppress stderr for git commands that are allowed to fail (missing preimage in --range and HEAD preimage lookup for new files).
  - Add explicit, deterministic warnings instead of raw git "fatal:" lines, e.g.:
    - `Manifest[N]: base preimage missing (new file at <base>): <path> (expected)`
  - Extend `scripts/validation/cor701-sha.mjs` to support range-mode hash capture (or add a new helper) so operators can compute correct Pre/Post values for `--range <a>..<b>` without ad-hoc scripts.
  - Clarify in docs/spec what COR-701 "SHA1" means (sha1 of content bytes, not git object id).
  - Update Coder/Validator protocol prompts/templates to explicitly mention:
    - allowlisted governance files that may appear in diffs (e.g., signature/gates json),
    - that raw git "fatal: path ... not in <rev>" lines can be non-fatal if the gate exits 0/PASS.
- OUT_OF_SCOPE:
  - Changing the COR-701 gate semantics (window/sha/line-delta rules) beyond improving message surfaces and helper tooling.
  - Removing strictness around new files (new files still must be fully manifested; preimage semantics stay deterministic).

## ACCEPTANCE_CRITERIA (DRAFT)
- Running `just post-work WP-{ID} --range <a>..<b>` on a WP that introduces new files:
  - Exits 0 when gates pass,
  - Does NOT print raw git stderr `fatal:` lines for "new file at base" preimage lookup,
  - Prints a deterministic warning line (or structured warning block) instead.
- Hash helper:
  - Provides a single supported command path to compute Pre/Post for `--range` mode, including new-file behavior.
  - Documentation clearly differentiates:
    - COR-701 "SHA1" (sha1 of bytes, LF-normalized when appropriate),
    - git object hashes (not used by COR-701 manifest fields).
- Agent friendliness:
  - Coder/Validator can mechanically treat PASS as PASS without manual interpretation of scary stderr noise.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Requires agreement on the normative behavior (Spec Enrichment) because this is not currently spelled out at the UX/message contract level.
- Must keep compatibility with Windows (CRLF/LF) behavior and existing validated WPs.

## RISKS / UNKNOWNs (DRAFT)
- Risk: accidentally suppressing stderr for genuinely unexpected failures. Mitigation: only suppress stderr for explicitly allow-fail git lookups; keep strict errors for unexpected commands.
- Risk: breaking existing scripts that rely on current stderr printing. Mitigation: add explicit warnings and keep exit codes unchanged; add tests (if repo has a place for validation script tests).

## SPEC_ENRICHMENT_NOTES (DRAFT - TO PREVENT CONTEXT LOSS)
Proposed spec addendum points (wording draft):
1) `post-work` MUST NOT emit raw toolchain stderr that resembles hard failure when the gate succeeds; gate output must be human- and machine-readable.
2) In `--range <base>..<head>` mode:
   - For files not present at base (new files), the "preimage" is treated as empty and is represented by the empty SHA1 `da39a3ee5e6b4b0d3255bfef95601890afd80709`.
   - This condition MUST be surfaced as a deterministic warning (not raw git fatal).
3) COR-701 SHA1 fields are sha1 of content bytes (with the repo's LF normalization rules), and MUST NOT be confused with git object IDs.
4) Provide one canonical helper command for computing COR-701 Pre/Post in both staged mode and range mode.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `docs/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `docs/refinements/WP-1-Post-Work-Check-Noise-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Post-Work-Check-Noise-v1` (in `docs/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `docs/TASK_BOARD.md` entry from STUB to Ready for Dev.
