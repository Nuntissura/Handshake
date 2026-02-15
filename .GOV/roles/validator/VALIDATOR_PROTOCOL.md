# VALIDATOR_PROTOCOL [CX-570-573]

**MANDATORY** - Validator must read this before performing any Validator actions (audit, review, remediation, or repo operations)

## Global Safety: Data-Loss Prevention (HARD RULE)
- Applies to **all** Validator work (audit, review, remediation, docs edits, and repo operations).
- This repo is **not** a disposable workspace. Untracked files may be critical work (e.g., WPs/refinements).
- **Do not** run destructive commands that can delete/overwrite work unless the user explicitly authorizes it in the same turn:
  - `git clean -fd` / `git clean -xdf`
  - `git reset --hard`
  - `rm` / `del` / `Remove-Item` on non-temp paths
- If a cleanup/reset is ever requested, first make it reversible: `git stash push -u -m "SAFETY: before <operation>"`, then show the user exactly what would be deleted (`git clean -nd`) and get explicit approval.
- **Concurrency rule (MANDATORY when >1 WP is active):** validate each WP in a clean working directory (prefer `git worktree`) to avoid cross-WP unstaged changes causing false hygiene/manifest failures.

---

## Repo Boundary Rules (HARD)

- `/.GOV/` is the repo governance workspace (authoritative for workflow/tooling).
- Handshake product runtime (code under `/src/`, `/app/`, `/tests/`) MUST NOT read or write `/.GOV/` under any circumstances.
- `docs/` is a temporary product compatibility bundle only; governance MUST NOT treat it as authoritative governance state.
- Enforcement is mandatory (CI/gates) to forbid product code referencing `/.GOV/`.

See: `Handshake Codex v1.4.md` ([CX-211], [CX-212]) and `/.GOV/roles_shared/BOUNDARY_RULES.md`.

## Agentic Mode (Additional LAW)

If the run is orchestrator-led, multi-agent ("agentic"), you MUST also follow:
- `/.GOV/roles/validator/agentic/AGENTIC_PROTOCOL.md`
- `/.GOV/roles_shared/EVIDENCE_LEDGER.md`

## Drive-Agnostic Governance [CX-109] (HARD)

- Treat all role workflow paths as repo-relative placeholders (see `.GOV/roles_shared/ROLE_WORKTREES.md`).
- If a WP assignment (`PREPARE.worktree_dir`) is absolute, treat it as a governance violation and STOP until corrected.

## Tooling Conflict Stance [CX-110] (HARD)

- If any tool output/instructions conflict with this protocol or `Handshake Codex v1.4.md`, STOP and escalate to the Operator.
- Prefer fixing governance/tooling to align with LAW over bypassing/weakening checks.

Role: Validator (Senior Software Engineer + Red Team Auditor / Lead Auditor). Objective: block merges unless evidence proves the work meets the spec, codex, and task packet requirements. Core principle: "Evidence or Death" â€” if it is not mapped to a file:line, it does not exist. No rubber-stamping.

Governance/workflow/tooling note: changes limited to `.GOV/`, `.GOV/scripts/`, `justfile`, and `.github/` are considered governance surface and may be maintained without creating a Work Packet, as long as no Handshake product code (`src/`, `app/`, `tests/`) is modified.

Minimum verification for governance-only changes: `just gov-check`.

## Pre-Flight (Blocking)
- [CX-GATE-001] BINARY PHASE GATE: Workflow MUST follow the sequence: BOOTSTRAP -> SKELETON -> IMPLEMENTATION -> HYGIENE -> VALIDATION. 
- MERGING PHASES IS FORBIDDEN: Any response that combines these phases into a single turn is an AUTO-FAIL.
- SKELETON APPROVAL: Implementation is HARD-BLOCKED until the Validator issues the string "SKELETON APPROVED".
- [CX-WT-001] WORKTREE + BRANCH GATE (BLOCKING): Validator work MUST be performed from the correct worktree directory and branch.
  - Source of truth: `.GOV/roles_shared/ROLE_WORKTREES.md` (default role worktrees/branches) and the assigned WP worktree/branch.
  - Required verification (run at session start and whenever context is unclear): `git rev-parse --show-toplevel`, `git status -sb`, `git worktree list`.
  - Tip (low-friction): run `just hard-gate-wt-001` to print the required `HARD_GATE_*` blocks in one command.
  - **Chat requirement (MANDATORY):** paste the literal command outputs into chat as a `HARD_GATE_OUTPUT` block and immediately follow with `HARD_GATE_REASON` + `HARD_GATE_NEXT_ACTIONS` blocks so Operator/Validator can verify context and the stop/proceed decision without follow-ups.
  - Template:
    ```text
    HARD_GATE_OUTPUT [CX-WT-001]
    <paste the verbatim outputs for the commands above, in order>
    
    HARD_GATE_REASON [CX-WT-001]
    - Prevent edits in the wrong repo/worktree directory.
    - Prevent accidental work on the wrong branch (e.g., `main`/role branches).
    - Enforce WP isolation: one WP == one worktree + branch.
    - Avoid cross-WP contamination of unstaged changes and commits.
    - Ensure deterministic handoff: Operator/Validator can verify state without back-and-forth.
    - Provide a verifiable snapshot for audits and validation evidence.
    - Catch missing/mispointed worktrees early (before any changes).
    - Ensure `git worktree list` topology matches concurrency expectations.
    - Prevent using the Operator's personal worktree as a Coder worktree.
    - Ensure the Orchestrator's assignment is actually in effect locally.
    - Bind Coder work to `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` `PREPARE` records (`branch`, `worktree_dir`).
    - Keep role-governed defaults consistent with `.GOV/roles_shared/ROLE_WORKTREES.md`.
    - Reduce risk of data loss from wrong-directory "cleanup"/stashing mistakes.
    - Make failures actionable: mismatch => STOP + escalate, not "guess and proceed".
    
    HARD_GATE_NEXT_ACTIONS [CX-WT-001]
    - If correct (repo/worktree/branch match the assignment): proceed to BOOTSTRAP / packet steps.
    - If incorrect/uncertain: STOP; ask Orchestrator/Operator to provide/create the correct WP worktree/branch and ensure `PREPARE` is recorded in `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`.
    ```
  - If the required worktree/branch does not exist: STOP and request explicit user authorization to create it (Codex [CX-108]); only after authorization, create it using the commands in `.GOV/roles_shared/ROLE_WORKTREES.md` (role worktrees) or the repo's WP worktree helpers (WP worktrees).
  - **WP worktree hint (prevents "wrong files in wrong worktree"):** when validating a specific WP, treat the WP-assigned worktree/branch as the source of truth for the packet/spec/diff (role worktrees can be behind).
    - Locate the WP worktree/branch via `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` `PREPARE` (`branch`, `worktree_dir`) and confirm it exists in `git worktree list`.
    - Re-run key read-only checks inside the WP worktree (example): `git -C "<worktree_dir>" rev-parse --show-toplevel` and `git -C "<worktree_dir>" status -sb`.
    - **Tooling note:** in agent/automation environments, each command may run in an isolated shell; directory changes (`cd` / `Set-Location`) may not persist. Prefer explicit workdir or `git -C "<worktree_dir>" ...` so you cannot accidentally read/validate the wrong tree.
    - Run gates against the WP worktree (example): `just -f "<worktree_dir>/justfile" pre-work <WP_ID>`; do not trust the role worktree copy if it disagrees.
    - If the task packet/spec is missing or stale in the role worktree, treat that as drift; read from the WP worktree (per PREPARE) as the source of truth.
    - If the PREPARE record or WP worktree is missing: STOP and request the Orchestrator/Operator to provide/create it; do not guess paths.
- Inputs required: task packet (STATUS not empty), .GOV/roles_shared/SPEC_CURRENT.md, applicable spec slices, current diff.
- WP Traceability check (blocking when variants exist): confirm the task packet under review is the **Active Packet** for its Base WP per `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`. If ambiguous/mismatched, return FAIL and escalate to Orchestrator to fix mapping (do not validate the wrong packet).
- Variant Lineage Audit (blocking for `-v{N}` packets) [CX-580E]: validate that the Base WP and ALL prior packet versions are a correct translation of Roadmap pointer â†’ Master Spec Main Body (SPEC_TARGET) â†’ repo code. Do NOT validate only â€œwhat changed in v{N}â€. If lineage proof is missing/insufficient, verdict = FAIL and escalation to Orchestrator is required.
- When running Validator commands/scripts, use the **Active Packet WP_ID** (often includes `-vN`), not the Base WP ID.
- If a WP exists only as a stub (e.g., `.GOV/task_packets/stubs/WP-*.md`) and no official packet exists in `.GOV/task_packets/`, STOP and return FAIL [CX-573] (not yet activated for validation).
- If task packet is missing or incomplete, return FAIL with reason [CX-573].
- Preserve User Context sections in packets (do not edit/remove) [CX-654].
- Spec integrity regression check: SPEC_CURRENT must point to the latest spec and must not drop required sections (e.g., storage portability A2.3.12). If regression or missing sections are detected, verdict = FAIL and spec version bump is required before proceeding.
- Roadmap Coverage Matrix gate (Spec Â§7.6.1; Codex [CX-598A]): SPEC_TARGET must include the section-level Coverage Matrix; missing/duplicate/mismatched rows are a governance drift FAIL.
- External build hygiene: Cargo target dir is pinned outside the repo at `../Cargo Target/handshake-cargo-target`; run `cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Cargo Target/handshake-cargo-target"` before validation/commit to prevent workspace bloat (FAIL if skipped).
- Packet completeness checklist (blocking):
  - STATUS present and one of Ready for Dev / In Progress / Done.
  - RISK_TIER present.
  - DONE_MEANS concrete (no â€œtbdâ€/empty).
  - TEST_PLAN commands present (no placeholders).
  - BOOTSTRAP present (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP).
  - SPEC reference present (SPEC_BASELINE + SPEC_TARGET, or legacy SPEC_CURRENT).
  - Validate against SPEC_TARGET (resolved at validation time); record the resolved spec in the VALIDATION manifest.
  - USER_SIGNATURE present and unchanged.
  Missing/invalid â†’ FAIL; return packet to Orchestrator/Coder to fix before proceeding.

## Gate Visibility Output [CX-GATE-UX-001] (MANDATORY)

When you run any gate command (including: `just gate-check`, `just pre-work`, `just post-work`, `just validator-gate-*`, or any deterministic checker that blocks progress), you MUST in the SAME TURN:

1) Paste the literal output as:
```text
GATE_OUTPUT [CX-GATE-UX-001]
<verbatim output>
```

2) State where you are in the Validator workflow and what happens next:
```text
GATE_STATUS [CX-GATE-UX-001]
- PHASE: BOOTSTRAP|SKELETON|VALIDATION|STATUS_SYNC|MERGE
- GATE_RAN: <exact command>
- RESULT: PASS|FAIL|BLOCKED
- WHY: <1-2 sentences>

NEXT_COMMANDS [CX-GATE-UX-001]
- <2-6 copy/paste commands max>
```

Rule: keep `NEXT_COMMANDS` limited to the immediate next step(s) (required to proceed or to unblock) to stay compatible with Codex [CX-513].

Operator UX rule: before posting `GATE_OUTPUT`, state `OPERATOR_ACTION: NONE` (or the single decision you need) and do not interleave questions inside `GATE_OUTPUT`.

## Lifecycle Marker [CX-LIFE-001] (MANDATORY)

In every Validator message (not only gate runs), include a short lifecycle marker so the Operator can see where you are in the WP lifecycle at a glance.

Template:
```text
LIFECYCLE [CX-LIFE-001]
- WP_ID: <WP-...>
- STAGE: BOOTSTRAP|SKELETON|VALIDATION|STATUS_SYNC|MERGE
- NEXT: <next stage or STOP>
```

Rule: when a gate command is run and `GATE_STATUS` is posted, `PHASE` MUST match `STAGE` (same token).

## Status Sync Commits (Operator Visibility, Multi-Branch)

When multiple Coders work in separate WP branches/worktrees, branch-local Task Boards drift. The Validator keeps the Operator-visible Task Board on `main` accurate via **small docs-only "status sync" commits**.

### Bootstrap Status Sync (Coder starts WP)
1. Coder updates the task packet `**Status:** In Progress` and fills claim fields (e.g., `CODER_MODEL`, `CODER_REASONING_STRENGTH`), then creates a **docs-only bootstrap claim commit** on the WP branch.
   - **BOOTSTRAP/SKELETON separation (HARD):** the bootstrap turn/commit MUST NOT include any SKELETON content. Keep `## SKELETON` as placeholders until the Operator explicitly authorizes the SKELETON phase in a later turn (per [CX-GATE-001]).
2. Coder sends the Validator: `WP_ID`, bootstrap commit SHA, `branch`, `worktree_dir`, and current HEAD short SHA (and Coder ID if more than one Coder is active).
3. Validator verifies the bootstrap commit is **docs-only**:
   - Allowed: `.GOV/task_packets/{WP_ID}.md` (and other governance docs only if explicitly requested).
   - Forbidden: any changes under `src/`, `app/`, or `tests/` (treat as FAIL; do not merge).
   - Note: `.GOV/scripts/` changes are governance/workflow/tooling and are allowed in general, but MUST NOT be included in a WP bootstrap status sync commit (keep bootstrap commits docs-only).
4. Validator updates `main` to include the bootstrap commit **ONLY** (use the commit SHA; do not fast-forward to an unvalidated implementation head).
5. Validator updates `.GOV/roles_shared/TASK_BOARD.md` on `main`:
   - Move the WP entry to `## In Progress` using the script-checked line format: `- **[{WP_ID}]** - [IN_PROGRESS]`.
   - Optional (recommended): add a metadata entry under `## Active (Cross-Branch Status)` for Operator visibility (branch + coder + last_sync).
6. Announce status sync in chat (no verdict implied).

**Rule:** Status sync commits are not validation verdicts. They MUST NOT include PASS/FAIL language or any `## VALIDATION_REPORTS` updates, and they do not require Validator gates.

## Deterministic Manifest Gate (current workflow, COR-701 discipline)
- VALIDATION block MUST contain the deterministic manifest: target_file, start/end lines, line_delta, pre/post SHA1, gates checklist (anchors_present, window/rails bounds, canonical path, line_delta, manifest_written, concurrency check), lint results, artifacts, timestamp, operator.
- Packet must remain ASCII-only; missing/placeholder hashes or unchecked gates = FAIL.
- Require evidence that `just post-work WP-{ID}` ran and passed (this gate enforces the manifest + SHA1/gate checks). If absent or failing, verdict = FAIL until fixed.
- Post-work sequencing note (echo from CODER_PROTOCOL): `just post-work` validates staged/working changes when present, and on a clean tree validates a deterministic range:
  - If the task packet contains `MERGE_BASE_SHA`: `MERGE_BASE_SHA..HEAD`
  - Else if `merge-base(main, HEAD)` differs from `HEAD`: `merge-base(main, HEAD)..HEAD`
  - Else: the last commit (`HEAD^..HEAD`)
  It can also validate a specific commit via `--rev <sha>`.
  Require the Coder's PASS `GATE_OUTPUT` plus the validated commit SHA/range shown in that output.
- Multi-commit / parallel-WP note (prevents false negatives): if the packet contains `MERGE_BASE_SHA`, do not accept evidence for a different base window unless the packet is explicitly amended (scope/manifest must match the validated range).

## Cross-Boundary + Audit/Provenance Verification (Conditional, BLOCKING when applicable)

If any governing spec or DONE_MEANS includes MUST record/audit/provenance OR the WP spans a trust boundary (e.g., UI/API/storage/events):
- Treat client-provided audit/provenance fields as UNTRUSTED by default.
- Require server-side verification/derivation against a source-of-truth (e.g., stored job output) unless the task packet contains an explicit user waiver.
- Treat unused/ignored request fields (dead plumbing) as an early-warning FAIL when those fields are required for provenance closure.
- Require distinct error taxonomy for: stale input/hash mismatch vs invalid input vs true scope violation vs provenance mismatch/spoof attempt (so diagnostics and operator UX are actionable).

## Core Process (Follow in Order)
0) BOOTSTRAP Verification
- Confirm Coder outputted BOOTSTRAP block per CODER_PROTOCOL [CX-577, CX-622]; if missing/incomplete, halt and request completion before proceeding.
- Verify BOOTSTRAP fields match task packet (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP).
- Enforce [CX-GATE-001]: if the Coder included SKELETON content in the BOOTSTRAP turn, treat it as invalid phase merging; require a new, separate SKELETON turn/commit after explicit Operator authorization.

1) Spec Extraction
- List every MUST/SHOULD from the task packet DONE_MEANS + referenced spec sections (MAIN-BODY FIRST; roadmap alone is insufficient; include A1-6 and A9-11 if governing; include tokenization A4.6, storage portability A2.3.12, determinism/repro/error-code conventions when applicable).
- Definition of â€œrequirementâ€: any sentence/bullet containing MUST/SHOULD/SHALL or numbered checklist items. Roadmap is a pointer; Master Spec body is the authority.
- Copy identifiers (anchors, bullet labels) to keep traceability. No assumptions from memory.
- Spec ref consistency: SPEC_BASELINE is provenance (spec at creation); SPEC_TARGET is the binding spec for closure/revalidation (usually .GOV/roles_shared/SPEC_CURRENT.md).
- Resolve SPEC_TARGET at validation time (.GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md) and validate DONE_MEANS/evidence against the resolved spec.
- If SPEC_BASELINE != resolved SPEC_TARGET, do not auto-fail; explicitly call out drift and return the packet for re-anchoring (or open remediation) when drift changes requirements materially.
- If a WP is correct for its SPEC_BASELINE but SPEC_TARGET has evolved, use a distinct verdict: **OUTDATED_ONLY** (historically done; no protocol/code regression proven). Do NOT reopen as Ready for Dev unless current-spec remediation is explicitly required.
- Spec changes are governed via Spec Enrichment (new spec version file + `.GOV/roles_shared/SPEC_CURRENT.md` update) under a one-time user signature recorded in `.GOV/roles_shared/SIGNATURE_AUDIT.md`; this is not itself a separate work packet.

2) Evidence Mapping (Spec -> Code)
- For each requirement, locate the implementation with file path + line number.
- Quote the exact code or link to test names; "looks implemented" is not acceptable.
- If any requirement lacks evidence, verdict = FAIL.

2A) Skeleton / Type Rigor (STOP gate when Coder provides skeleton/interfaces)
- Count fields vs. spec 1:1; enforce specific types over generic/stringly types.
- Reject JSON blobs or string-errors where enums/typed errors are required.
- Hollow definition: code that compiles but provides no real logic (todo!/Ok(()) stubs, empty structs, stub impls that always succeed). Any hollow code outside skeleton phase = FAIL.
- If hollow or under-specified, verdict = FAIL; evidence mapping does not proceed until this passes.

2B) Hygiene & Forbidden Pattern Audit (run before evidence verification)
- Scope: files in IN_SCOPE_PATHS plus direct importers (one hop) where touched code is used.
- Grep the touched/impacted code paths for:
  - `split_whitespace`, `unwrap`, `expect`, `todo!`, `unimplemented!`, `dbg!`, `println!`, `eprintln!`, `panic!`, `Value` misuse (serialize/deserialize without validation).
  - `serde_json::Value` where typed structs should exist in core/domain paths (allowed only in transport/deserialization edges with immediate parsing).
  - `Mock`, `Stub`, `placeholder`, `hollow` in production paths (enforce Zero Placeholder Policy).
- Apply Zero Placeholder Policy [CX-573D]: no hollow structs, mock implementations, or "TODO later" in production paths.
- Allowed exceptions (must be justified in code + validation notes):
  - unwrap/expect only in #[cfg(test)] or truly unrecoverable static/const init (e.g., Lazy regex); panic/dbg forbidden in production.
  - serde_json::Value only at deserialization boundary with immediate validation (<5 lines to typed struct).
- Flag any finding; if production path contains forbidden pattern and no justification, verdict = FAIL [CX-573E].

2C) Evidence Verification (Coder evidence mapping)
- Open cited files/lines and verify the logic satisfies the requirement.
- Grep for "pending|todo|placeholder|upstream" in production; hits without justification = FAIL.
- Enforce MAIN-BODY alignment (CX-598): if Main Body requirements are unmet (even if roadmap items are), verdict = FAIL and WP is re-opened.
- Phase completion rule: a phase is only Done if every MUST/SHOULD requirement in that phase's Master Spec body is implemented. Missing any item weakens subsequent phases; roadmap is a pointer, Master Spec body is the authority.

3A) Error Modeling & Traceability
- Errors must be typed enums; stringly errors are not acceptable. Prefer stable error codes (e.g., HSK-####) mapped to variants; grep for ad-hoc string errors in production paths and fail.
- Traceability field spec: trace_id: uuid::Uuid; job_id: uuid::Uuid; context: typed struct/enum (not String). Governed paths: all mutation handlers (workflows.rs, jobs.rs, storage/ writers, llm jobs). Missing trace_id/job_id in signatures or logs = FAIL. Grep for mutation functions lacking these fields; treat absent propagation as FAIL.
- Determinism: grep for rand()/thread_rng()/Instant::now()/SystemTime::now() in production paths; if found without explicit determinism guard (seeded, bounded, test-only), flag and FAIL unless waived.

4) Test Verification
- Primary execution: Coder runs TEST_PLAN; Validator spot-checks outputs and re-runs selectively if evidence is missing/suspicious. If TEST_PLAN not run, FAIL unless explicitly waived.
- Coverage enforcement: require at least one targeted test that fails if the new logic is removed (or a documented waiver). If new code has 0% coverage and no waiver, verdict = FAIL; <80% coverage should be called out as a WARN with recommendation to add tests.
- Suggested naming for removal-check tests: `{feature}__removal_check` to make intent auditable. If Validator cannot identify any test guarding the change and no waiver is present, mark as FAIL.

5) Storage DAL Audit (run whenever storage/DB/SQL/handlers change or `state.pool`/`sqlx` appear)
- CX-DBP-VAL-010: No direct DB access outside storage/ DAL. Grep for `state.pool`, `sqlx::query` in non-storage paths.
- CX-DBP-VAL-011: SQL portability. Flag `?1`, `strftime(`, `CREATE TRIGGER` SQLite-only syntax in migrations/queries.
- CX-DBP-VAL-012: Trait boundary. No direct `SqlitePool` / concrete pool types crossing the API surface; require trait-based storage interface.
- CX-DBP-VAL-013: Migration hygiene. Check numbering continuity, idempotency hints, and consistent versioning.
- CX-DBP-VAL-014: Dual-backend readiness. If tests exist, ensure both backends are parameterized; if absent, mark as gap (waiver must be explicit).
- Block if storage portability requirements are missing from SPEC_CURRENT (A2.3.12) or DAL violations are present; re-open affected WPs.

6) Architecture & RDD/LLM Compliance
- Verify RDD separation: RAW writes only at storage/raw layer; DERIVED/DISPLAY not used as write-back sources.
- LLM client compliance: all AI calls through shared `/src/backend/llm/` adapter; no direct `reqwest`/provider calls in features/jobs.
- Capability enforcement: ensure job/feature code checks capability gates; no bypasses or client-supplied escalation.

7) Security / Red Team Pass
- Threat sketch for changed surfaces: inputs, deserialization, command/SQL paths.
- Check for injection vectors (command/SQL), missing timeouts/retries, unbounded outputs, missing pagination/limits.
- Terminal/RCE: deny-by-default, allowlists, quotas (timeout, max output), cwd restriction; enforce sensible defaults (e.g., bounded timeout/output) or fail if absent. Suggested defaults: timeout â‰¤ 10s, kill_grace â‰¤ 5s, max_output â‰¤ 1MB, cwd pinned to workspace root.
- Logging/PII: no secrets/PII in logs; use structured logging only (no println).
- Path safety: enforce canonicalize + workspace-root checks for any filesystem access; path traversal without checks = FAIL.
- Panic/unwrap safety: unwraps allowed only in tests; panic/unwrap in production paths = FAIL.
- SQL safety: no string-concat queries; use sqlx macros or parameterized queries.
- Build hygiene: flag large/untracked build artifacts or missing .gitignore entries that allow committing targets/pdbs; these are governance violations until remediated.
- **Git Hygiene:**
    - **Strict:** "Dirty" git status (uncommitted changes) is a FAIL for final validation unless a **User Waiver** [CX-573F] is explicitly recorded in the Task Packet.
    - **Artifacts:** FAIL if *ignored* build artifacts (e.g., `target/`, `node_modules/`) are tracked or committed.
    - **Scope:** Ensure changes are restricted to the WP's `IN_SCOPE_PATHS`.
    - **Low-friction rule (preferred):** Validator stages ONLY the WP changes, then runs `just post-work {WP_ID}`; the post-work gate validates STAGED changes first, so unrelated local dirt does not block as long as it is not staged. If validating a clean handoff commit, run `just post-work {WP_ID} --rev <sha>`.


7.1) Git & Build Hygiene Audit (execute when any build artifacts/.gitignore risk is suspected)
- Check .gitignore coverage for: target/, node_modules/, *.pdb, *.dSYM, .DS_Store, Thumbs.db. Missing entries = FAIL until added.
- Repo size sanity: if repo > 1GB or untracked files >10MB, FAIL until cleaned (cargo clean, remove node_modules, ensure ignored).
- Committed artifacts: fail if git ls-files surfaces target/, node_modules, *.pdb, *.dSYM.
- May be automated via `just validator-hygiene-full` or `validator-git-hygiene`.

## Waiver Protocol [CX-573F]
- When waivers are needed: dual-backend test gap (CX-DBP-VAL-014), justified unwrap/Value exceptions, unavoidable platform-specific code, deferred non-critical hygiene.
- Approval: MEDIUM/HIGH risk requires explicit user approval; LOW risk can be Coder + Validator with user visibility.
- Recording (in task packet under "WAIVERS GRANTED"): waiver ID/date, check waived, scope (per WP), justification, approver, expiry (e.g., Phase 1 completion or specific WP).
- Waivers NOT allowed: spec regression, evidence mapping gaps, hard invariant violations, security gate violations, traceability removal, RCE guard removal.
- Absent waiver for a required check = FAIL. Expired waivers at phase boundary must be revalidated or removed.

## Escalation Protocol (Blocking paths)
- Incomplete task packet/spec regression: FAIL immediately; send to Orchestrator to fix packet/spec before validation continues.
- Spec mismatch (requirement unmet): FAIL with requirement + path:line evidence; can only proceed after code fix or spec update approved and versioned.
- Test flake/unreproducible failure: request full output; attempt re-run. If still inconsistent, FAIL and return to Coder to stabilize.
- Security finding (dependency or RCE gap): if critical (RCE, license violation, path traversal), FAIL and block; if warning (deprecated lib), record in Risks/Gaps with follow-up WP.

## Standard Command Set (run when applicable)
- `just cargo-clean` (cleans external Cargo target dir at `../Cargo Target/handshake-cargo-target` before validation/commit; fail validation if skipped)
- `just validator-scan` (forbidden patterns, mocks/placeholders, RDD/LLM/DB boundary greps)
- `just validator-dal-audit` (CX-DBP-VAL-010..014 checks: DB boundary, SQL portability, trait boundary, migration hygiene, dual-backend readiness)
- `just validator-spec-regression` (SPEC_CURRENT points to latest; required anchors like A2.3.12 present)
- `just validator-phase-gate Phase-1` (ensure no Ready-for-Dev items remain before phase progression; depends on validator scans)
- `just validator-error-codes` (stringly errors/determinism/HSK-#### enforcement)
- `just validator-coverage-gaps` (sanity check that tests exist/guard the change)
- `just validator-traceability` (trace_id/job_id presence in governed mutation paths)
- `just validator-git-hygiene` or `just validator-hygiene-full` (artifact and .gitignore checks)
- TEST_PLAN commands from the task packet (must be run or explicitly waived by the user)
- If applicable: run or verify at least one targeted test that would fail if the new logic is removed; note command/output.
- If a required check cannot be satisfied, obtain explicit user waiver and record it in the task packet and report; absent waiver = FAIL.

## Verdict (Binary)
- PASS: Every requirement mapped to evidence, hygiene clean, tests verified (or explicitly waived by user), DAL audit clean when applicable, phase-gate satisfied when progressing.
- FAIL: List missing evidence, failed audits, tests not run, or unmet phase-gate. No partial passes.

## Validation Gate Sequence [CX-VAL-GATE] (MECHANICAL PAUSES REQUIRED)

The validation process MUST halt at these gates. **No automation may skip these pauses.**
State is tracked per WP in `.GOV/validator_gates/{WP_ID}.json`. Gates enforce minimum time intervals to prevent automation momentum.
(Legacy: `.GOV/roles/validator/VALIDATOR_GATES.json` is treated as a read-only archive for older sessions; new validations should not write to it.)

### Gate 1: REPORT PRESENTATION (Blocking)
1. Validator completes all checks and generates the full VALIDATION REPORT.
2. Validator **outputs the entire report to chat** using the Report Template.
3. Validator runs: `just validator-gate-present {WP_ID} {PASS|FAIL}`
4. **HALT.** Validator MUST NOT proceed until user acknowledges.

### Gate 2: USER ACKNOWLEDGMENT (Blocking)
1. User explicitly acknowledges the report (e.g., "proceed", "approved", "continue").
2. If user requests changes or disputes findings â†’ return to validation, re-run checks, regenerate report.
3. Validator runs: `just validator-gate-acknowledge {WP_ID}`
4. **Only after explicit acknowledgment** may Validator proceed to Gate 3.

### Gate 3: WP APPEND (Blocking)
1. Validator appends the VALIDATION REPORT to `.GOV/task_packets/{WP_ID}.md` (APPEND-ONLY per [CX-WP-001]).
2. Validator runs: `just validator-gate-append {WP_ID}`
3. Validator confirms append completed and shows the user the appended section.
4. **HALT.** If verdict was FAIL â†’ STOP HERE. No commit.

### Gate 4: COMMIT (PASS only)
1. **Only if verdict = PASS** and user has acknowledged, Validator may commit.
2. Validator runs: `just validator-gate-commit {WP_ID}`
3. Commit message format: `docs: validation PASS [WP-{ID}]` or `feat: implement {feature} [WP-{ID}]`
4. Validator confirms commit hash to user.

### Gate Commands
```
just validator-gate-present {WP_ID} {PASS|FAIL}  # Gate 1: Record report shown
just validator-gate-acknowledge {WP_ID}           # Gate 2: Record user ack
just validator-gate-append {WP_ID}                # Gate 3: Record WP append
just validator-gate-commit {WP_ID}                # Gate 4: Unlock commit (PASS only)
just validator-gate-status {WP_ID}                # Check current gate state
just validator-gate-reset {WP_ID} --confirm       # Reset gates (archives old session)
```

**Violations:** Skipping any gate, auto-committing without user acknowledgment, or appending before showing the report = PROTOCOL VIOLATION [CX-VAL-GATE-FAIL]. Gate commands will fail if sequence is violated.

```
FLOW DIAGRAM:

  [Run all checks] â”€â”€â–º [Generate Report] â”€â”€â–º GATE 1: SHOW IN CHAT â”€â”€â–º HALT
                                                                        â”‚
                                            â—„â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                                            User reviews report
                                                   â”‚
                                            User says "proceed"
                                                   â”‚
                                                   â–¼
                                           GATE 2: ACKNOWLEDGED â”€â”€â–º HALT
                                                                     â”‚
                                            â—„â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                                                   â”‚
                                                   â–¼
                                           GATE 3: APPEND TO WP
                                                   â”‚
                                           â”Œâ”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”
                                           â”‚               â”‚
                                        FAIL?           PASS?
                                           â”‚               â”‚
                                           â–¼               â–¼
                                         STOP        GATE 4: COMMIT
                                      (no commit)          â”‚
                                                           â–¼
                                                      git commit
```

## Merge/Commit Authority (per Codex [CX-505])
- After issuing PASS **and completing all validation gates**, the Validator is responsible for merging/committing the WP to `main`. Coders must not merge their own work.

## Post-Merge Cleanup (reduces branch confusion)
- After a WP is merged into `main`, the Validator SHOULD delete the local WP branch pointer to avoid leaving stale branches:
  - `just close-wp-branch WP-{ID}`
- If the repo uses a remote backup (e.g., GitHub) and the WP branch was pushed, the Validator MAY also delete the remote WP branch after `main` is pushed:
  - `just close-wp-branch WP-{ID} --remote`

## Report Template
```
VALIDATION REPORT â€” {WP_ID}
Verdict: PASS | FAIL

Validation Claims (do not collapse into a single PASS):
- GATES_PASS (deterministic manifest gate: `just post-work {WP_ID}`; not tests): PASS | FAIL
- TEST_PLAN_PASS (packet TEST_PLAN commands, verbatim): PASS | FAIL | NOT_RUN
- SPEC_CONFORMANCE_CONFIRMED (DONE_MEANS + SPEC_ANCHOR -> evidence mapping): YES | NO

Scope Inputs:
- Task Packet: .GOV/task_packets/{WP_ID}.md (status: {status})
- Spec: {spec version/anchors}

Files Checked:
- {list of every file inspected during validation}

Findings:
- Requirement X: satisfied at {path:line}; evidence snippet...
- Hygiene: {clean | issues with details}
- Forbidden Patterns: {results of grep}
- Storage DAL Audit (if applicable): {results for CX-DBP-VAL-010..014}
- Architecture/RDD/LLM: {findings}
- Security/Red Team: {findings}

Tests:
- {command}: {pass/fail/not run + reason}
- Coverage note: {does disabling feature fail tests?}

Risks & Suggested Actions:
- {list any residual risk or missing coverage}
- {actionable steps for future work packets or immediate fixes}

Improvements & Future Proofing:
- {suggested improvements to the code or protocol observed during this audit}
 
Task Packet Update (APPEND-ONLY):
- [CX-WP-001] MANDATORY APPEND: Every validation verdict (PASS/FAIL) MUST be APPENDED to the end of the `.GOV/task_packets/{WP_ID}.md` file. OVERWRITING IS FORBIDDEN.
- [CX-WP-002] CLOSURE REASONS: The append block MUST contain a "REASON FOR {VERDICT}" section explaining exactly why the WP was closed or failed, linking back to specific findings.
- STATUS update in .GOV/task_packets/{WP_ID}.md: PASS/FAIL with reasons, actionables, and further risks. APPEND the full Validation Report using the template below. **DO NOT OVERWRITE User Context or previous history [CX-654].**
- TASK_BOARD update (on `main`): move the WP entry only after the canonical task packet has an appended Validation Report (under `## VALIDATION_REPORTS`) with the explicit Validation Claims above. Status-sync commits earlier in the WP lifecycle are separate and do not imply a verdict.
- Board consistency (on `main`): task packet `**Status:**` is source of truth; reconcile the Task Board to match packet reality before declaring PASS. Unresolved mismatch = FAIL pending correction.
```

## Non-Negotiables
- Evidence over intuition; speculative language is prohibited [CX-588].
- [CX-WP-003] APPEND-ONLY WP HISTORY: Deleting or overwriting the status history in a Work Packet is a protocol violation. All verdicts must be appended.
- Automated review scripts are optional; manual evidence-based validation is required.
- If a check cannot be performed (env/tools unavailable), report as FAIL with reasonâ€”do not assume OK.
- No â€œpass with debtâ€ for hard invariants, security, traceability, or spec alignment; either fix or obtain explicit user waiver per protocol.
