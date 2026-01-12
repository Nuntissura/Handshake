# 02) Artifacts and Contracts (Kernel)

This kernel is built around a small set of **canonical artifacts** (files) that jointly answer:
- What is the current authoritative spec?
- What work is authorized, and with what scope?
- What evidence exists, and what gates remain?
- What is the current project state (WPs in progress/done/stub)?

The primary design constraint: a **fresh small-context agent** must be able to reconstruct state by opening a short, stable set of files.

## A. Global invariants (apply to every artifact unless stated otherwise)

### A1) Deterministic parsing
If an artifact is read by gate tooling, it MUST be deterministic to parse:
- Prefer ASCII-only for parser-facing artifacts (task packets, refinements). If non-ASCII is unavoidable, escape as `\\uXXXX`.
- Avoid relying on human-only meaning (e.g., “we all know what this means”).
- Avoid ambiguous formatting (mixed heading styles, inconsistent field labels).

### A2) Canonical naming (portable across projects)
The governance system is portable only if filenames are predictable.

Minimum conventions:
- **WP IDs** MUST be stable identifiers (no timestamps in filenames).
- Packet revisions MUST use `-vN` suffix (example: `WP-12-Foo-v3`).
- When revisions exist, the system MUST record which packet is active (see `docs/WP_TRACEABILITY_REGISTRY.md`).

### A3) Append-only audit logs
Audit logs (e.g., signatures) MUST be append-only and treated as evidence, not narrative.

## B. Canonical repo layout (kernel default)

This kernel assumes these stable locations:

- `AGENTS.md` (repo root): repo-local agent hard rules.
- `<PROJECT> Codex vX.Y.md` (repo root): project constitution for agents/humans.
- `<PROJECT>_Master_Spec_vNN.NNN.md` (repo root): authoritative spec versions.
- `docs/` (governance surface)
  - `docs/START_HERE.md` (entrypoint; optional but recommended)
  - `docs/SPEC_CURRENT.md` (pointer to current Master Spec)
  - `docs/TASK_BOARD.md` (global execution state)
  - `docs/WP_TRACEABILITY_REGISTRY.md` (Base WP -> Active Packet)
  - `docs/SIGNATURE_AUDIT.md` (append-only signature log)
  - `docs/ORCHESTRATOR_PROTOCOL.md`, `docs/CODER_PROTOCOL.md`, `docs/VALIDATOR_PROTOCOL.md`
  - `docs/ORCHESTRATOR_GATES.json`, `docs/VALIDATOR_GATES.json` (gate state)
  - `docs/templates/` (canonical templates)
  - `docs/refinements/` (approved refinements)
  - `docs/task_packets/` (activated packets)
    - `docs/task_packets/stubs/` (backlog stubs; not executable)

Projects MAY add additional governance modules (runbooks, ADRs, rubrics). They must still be discoverable from stable entrypoints.

## C. Artifact contracts (required files)

### C1) `docs/SPEC_CURRENT.md` (spec pointer)
Purpose:
- Provides a single source of truth for the current authoritative Master Spec version.

Contract:
- Contains exactly one resolvable path to the current spec file (implementation-defined, but deterministic).
- Gate tooling MUST treat this as the only pointer; other docs must not “quietly override” it.

Failure modes if missing/wrong:
- Agents code against old specs; validators cannot reliably re-resolve intent at review time.

### C2) Master Spec files (`<PROJECT>_Master_Spec_vNN.NNN.md`)
Purpose:
- Centralizes product intent and normative requirements.

Kernel constraint:
- The Master Spec MUST be written to support anchoring (stable headings, stable section IDs, and “Main Body first” discipline if used).

Failure modes:
- Refinements cannot create stable anchors; WPs become “vibe-coded”.

### C3) `docs/TASK_BOARD.md` (execution state SSoT)
Purpose:
- Single source of truth for WP execution state across roles and models.

Contract (recommended minimum):
- Must contain explicit state sections (example: `## In Progress`, `## Done`, `## Superseded`, plus `## Stubs` if used).
- Each WP entry MUST include: `WP_ID`, Status, and link/path to the active task packet (directly or via traceability registry).

Failure modes:
- Parallel agents diverge on “what is active”; WPs are duplicated or silently dropped.

### C4) `docs/WP_TRACEABILITY_REGISTRY.md` (Base WP -> Active Packet)
Purpose:
- Resolves ambiguity when multiple packets exist for the same Base WP (revisions, superseded attempts).

Contract:
- For every Base WP with multiple packet files, record a single active packet path.
- Must be deterministic to parse (table or strict bullet format).

Failure modes:
- Validators/coders open the wrong revision; acceptance criteria drift across versions.

### C5) `docs/refinements/<WP_ID>.md` (refinement artifact)
Purpose:
- Captures the **Technical Refinement Block** that binds a WP to the Master Spec and makes scope executable.

Contract (kernel-level):
- ASCII-only.
- Must include:
  - `WP_ID`
  - `SPEC_TARGET_RESOLVED` (resolved pointer)
  - `SPEC_TARGET_SHA1` (hash of the resolved spec file at refinement time)
  - `USER_APPROVAL_EVIDENCE` (deterministic string used to prevent “momentum signatures”)
  - `USER_SIGNATURE` (Operator signature token)
  - One or more `SPEC_ANCHORS`, each with:
    - start line, end line, and a context token that must appear within that window in the resolved spec.
    - excerpt captured as ASCII (with `\\uXXXX` escapes if needed)
  - `CLEARLY_COVERS` checklist + verdict fields
  - `ENRICHMENT` decision + (if needed) copy-pastable proposed enrichment text

Failure modes:
- Packets lack binding to spec; validators cannot prove requirements are “in spec main body”.
- Small-context coders cannot reconstruct why the WP exists.

### C6) `docs/task_packets/stubs/<WP_ID>.md` (stub packets; non-executable)
Purpose:
- Maintains a backlog of future WPs without consuming signatures or producing enforceable scope.

Contract:
- Must clearly declare itself NON-EXECUTABLE (e.g., `STUB_STATUS: STUB`).
- Must not be used as authority for implementation/validation.
- Must include an activation checklist that references refinement/signature requirements.

Failure modes:
- Coders start work from stubs, bypassing refinement and scope gates.

### C7) `docs/task_packets/<WP_ID>.md` (activated task packets; executable authority)
Purpose:
- Single authoritative “work contract” for a coder, and the primary audit surface for validators.

Contract (minimum kernel requirements):
- ASCII-only.
- Stable required sections (case-insensitive heading match is allowed, but headings must exist):
  - `## METADATA` (must include `WP_ID`, `BASE_WP_ID`, `Status`, `USER_SIGNATURE`, and declared `ROLE` that authored the packet)
  - `## SCOPE` (must include explicit `IN_SCOPE_PATHS` and explicit `OUT_OF_SCOPE`)
  - `## QUALITY_GATE` with `TEST_PLAN` and `DONE_MEANS`
  - `## AUTHORITY` (must include spec pointer + codex + task board + traceability registry)
  - `## BOOTSTRAP` (files to open, search terms, commands)
  - `## SKELETON` (interface-first design)
  - `## IMPLEMENTATION` (coder fills only after skeleton approval gate)
  - `## HYGIENE`
  - `## VALIDATION` (mechanical manifest blocks for every changed non-doc file)
  - `## STATUS_HANDOFF`
  - `## EVIDENCE` (append logs/output)
  - `## VALIDATION_REPORTS` (validator append-only audits/verdicts)

Kernel phase gate requirement:
- A literal line containing exactly `SKELETON APPROVED` MUST exist outside fenced code blocks before any “implementation evidence” markers recognized by gate tooling.

Failure modes:
- Scope creep (“I also refactored X”) becomes unauditable.
- Post-work evidence cannot be validated (hashes/line windows missing).

### C8) Templates (`docs/templates/`)
Purpose:
- Makes artifact creation reproducible and reduces formatting drift that breaks gate tooling.

Contract:
- Canonical templates SHOULD be stored in `docs/templates/` and copied into new artifacts.
- If compatibility shims exist in `docs/` (legacy paths), they must be explicitly labeled as shims.

Failure modes:
- Gate scripts fail due to format drift; new models produce incompatible packets/refinements.

### C9) Gate state (`docs/ORCHESTRATOR_GATES.json`, `docs/VALIDATOR_GATES.json`)
Purpose:
- Stores the state machine for role-specific gates (refine/sign/prepare; validate/acknowledge/commit, etc.).

Contract:
- JSON is treated as authoritative gate state.
- Must be append-only in effect: state transitions are logged with timestamps and immutable evidence links.

Failure modes:
- Agents cannot prove which gates were completed; “verdicts” become social, not mechanical.

### C10) Signature audit log (`docs/SIGNATURE_AUDIT.md`)
Purpose:
- Central append-only log of Operator signatures and what they approved.

Contract:
- Each signature entry must link to the artifact(s) signed (refinement, packet).
- Format must be deterministic enough for tooling to confirm that a signature exists for a given WP.

Failure modes:
- Work can be started without real Operator authorization; approvals can be disputed.

