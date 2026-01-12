# Reference Implementation: Handshake Governance (Non-Normative)

Purpose: map a concrete repository (Handshake) to the project-agnostic governance kernel (`docs/GOV_KERNEL/*`), including an exhaustive inventory of governing artifacts and enforcement scripts.

Scope:
- This is a governance/operations spec, not product behavior.
- It documents (a) the kernel-level concepts and (b) how they are concretely implemented by files/scripts in this repo.
- "No file left out" means: every file under the governance surface (docs/, docs_local/, scripts/, .github/) plus all root-level governance/config files are enumerated in the inventory section.

Non-goals:
- Do not change product code (`src/`, `app/`, `tests/`) here.
- Do not treat this document as a replacement for the authoritative law stack (Codex + Master Spec + protocols); it is an implementation map and inventory.

---

## 1) Authority Stack (LAW) and Precedence

The governance system is explicit about precedence. The current implemented stack is:

1. `Handshake Codex v1.4.md` (repo root)
   - Defines repo invariants and allowed assistant behavior (including hard bans on destructive ops and git worktree/branch rewrites without explicit consent).
2. Master Spec: `Handshake_Master_Spec_v*.md` (repo root), with pointer file `docs/SPEC_CURRENT.md`
   - Product intent + architecture + normative requirements; "Main Body first" discipline is enforced mechanically.
3. Protocol layer in `docs/`
   - `docs/ORCHESTRATOR_PROTOCOL.md` (Orchestrator behavior and signature/refinement workflow)
   - `docs/CODER_PROTOCOL.md` (Coder behavior and phase gating)
   - `docs/VALIDATOR_PROTOCOL.md` (Validator behavior and evidence-based audit rules)
4. Repo guardrails: `AGENTS.md`
   - Local hard rules specific to this repo (no destructive cleanup; WP branching/worktree; checkpoint commit gate).
5. Mechanical enforcement scripts + `justfile`
   - Deterministic checks and workflow gates implemented as executable scripts under `scripts/` and invoked via `just`.

Important implemented constraint:
- The system is designed to support small-context local models by forcing work to be packetized, anchored, and gate-checked (see Sections 4 and 7).

---

## 2) Roles (Mechanical Separation of Duties)

This repo uses rigid roles that intentionally limit what each agent may do. The design goal is to prevent accidental scope creep, spec drift, and un-auditable changes.

### OPERATOR (human)
- Sets priorities and approves (in-chat) refinements and signatures.
- Owns any explicit overrides to governance.

### ORCHESTRATOR (lead architect / workflow manager)
- Creates and maintains governance artifacts: stubs, refinements, task packets, board, traceability, signature audit.
- Does not implement product code.
- Owns "spec-to-work translation" (SPEC_ANCHOR mapping, DONE_MEANS, TEST_PLAN, exact IN_SCOPE_PATHS).
- Runs Orchestrator gate scripts to record refinement/signature/prepare events.

### CODER (implementation)
- Implements only what the task packet requires, within IN_SCOPE_PATHS.
- Must not change scope or interpret spec beyond what the packet anchors.
- Must not claim validation verdicts (Validator-only).

### VALIDATOR (auditor / red team)
- Performs evidence-based review: opens files, maps requirements to file:line, verifies tests.
- Controls the final "PASS -> commit" gate via validator gate logs.
- Maintains operator-visible status sync on `main` (Active cross-branch section).

### Optional roles (supported by the docs but not always active)
- Debugger (triage; uses `docs/RUNBOOK_DEBUG.md`)
- Tooling agent (runs scripts / builds diagnostic bundles)
- Red-team framing exists as a responsibility inside Validator and Refinement blocks.

---

## 3) Canonical Governance Artifacts (What exists, and why)

This section describes the key governance artifacts and how they gate each other.

### 3.1 Navigation + orientation pack (`docs/`)
- `docs/START_HERE.md`: canonical entry point and command surface.
- `docs/ARCHITECTURE.md`: module map and allowed dependency boundaries.
- `docs/RUNBOOK_DEBUG.md`: incident/CI triage, log locations, minimal debug flow.
- `docs/PAST_WORK_INDEX.md`: archaeology pointers (note: this file currently contains stale references; see Section 8).

Why: enables a fresh model (or human) to orient quickly and deterministically without reading the whole repo.

### 3.2 Spec pointer and spec drift guard
- `docs/SPEC_CURRENT.md`: the single pointer to the current authoritative Master Spec.
- `scripts/spec-current-check.mjs`: enforces that SPEC_CURRENT points to the latest `Handshake_Master_Spec_v*.md` file by parsed version.

Why: prevents silent spec drift and "coding against an old spec".

### 3.3 Task Board (execution state SSoT)
- `docs/TASK_BOARD.md`: the global state tracker for Phase 1 WPs.
- `scripts/validation/task-board-check.mjs`: enforces strict formatting for `## In Progress`, `## Done`, `## Superseded` entries.

Key rule (enforced in docs and protocols):
- Task Board is intentionally minimal; detailed reasons live in packets.

### 3.4 Work Packet Traceability (Base WP -> Active Packet mapping)
- `docs/WP_TRACEABILITY_REGISTRY.md`: resolves Base WP IDs to an Active Packet file path, especially when revisions exist (`-vN`).

Why: the Master Spec should not embed revision packet IDs; this registry prevents ambiguity when multiple packet revisions exist.

### 3.5 Stubs vs activated packets
- Stubs: `docs/task_packets/stubs/` (not executable)
- Activated packets: `docs/task_packets/` (executable authority for implementation/validation)
- Templates:
  - `docs/templates/TASK_PACKET_STUB_TEMPLATE.md`
  - `docs/templates/TASK_PACKET_TEMPLATE.md`

Why: allows backlog reshaping without consuming signatures, while keeping "Ready for Dev" meaningfully executable.

### 3.6 Refinements (Technical Refinement Block artifacts)
- `docs/refinements/{WP_ID}.md`: per-WP refinement artifact.
- Template: `docs/templates/REFINEMENT_TEMPLATE.md`
- Mechanical enforcement: `scripts/validation/refinement-check.mjs`

Key implemented properties:
- ASCII-only.
- Includes SPEC_TARGET_RESOLVED + SPEC_TARGET_SHA1 binding to the current spec file.
- Includes SPEC_ANCHORS with excerpt window and token-in-window match requirements.
- Includes "CLEARLY_COVERS" 5-point checklist and ENRICHMENT decision.
- Includes `USER_APPROVAL_EVIDENCE` as a deterministic guard against momentum.

Why: makes spec anchoring checkable and portable across small-context models.

### 3.7 Signatures (one-time, auditable)
- `docs/SIGNATURE_AUDIT.md`: authoritative registry of signatures consumed.
- Orchestrator gate script appends entries and enforces one-time use.

Why: creates a forced alignment pause and prevents autonomous drift.

### 3.8 Gate state logs (machine-readable)
- `docs/ORCHESTRATOR_GATES.json`: log of REFINE/SIGN/PREPARE events.
- `docs/VALIDATOR_GATES.json`: log of validation gate sequence (present -> acknowledge -> append -> commit).

Why: provides deterministic, machine-checkable proof that the workflow was followed.

### 3.9 Quality gate definition
- `docs/QUALITY_GATE.md`: Gate 0 (pre-work) and Gate 1 (post-work) definitions; risk tier matrix; required commands.

Why: sets a minimum hygiene baseline; prevents "it compiled on my machine" merges.

### 3.10 Role-local worktree policy
- `docs/ROLE_WORKTREES.md`: local mapping of role -> (worktree dir, branch) on the operator machine.

Why: prevents role confusion and cross-WP contamination; makes "where am I?" checkable.

### 3.11 Ownership and agent identity
- `docs/OWNERSHIP.md`: area owners for review routing.
- `docs/agents/AGENT_REGISTRY.md`: agent IDs and role mapping.

Why: provides accountability for multi-agent work and review routing.

---

## 4) End-to-End Mechanical Workflow (How the gates interlock)

This section maps the workflow to concrete scripts and state files.

### 4.1 Global hard gate (environment + repo state)
Required for Orchestrator/Coder/Validator sessions:
- `pwd`
- `git rev-parse --show-toplevel`
- `git rev-parse --abbrev-ref HEAD`
- `git status -sb`
- `git worktree list`

Why: role work must occur in the correct worktree and branch, preventing accidental cross-role actions.

### 4.2 Backlog lifecycle: STUB -> Activated -> Ready for Dev
1. Create stub file in `docs/task_packets/stubs/` (no signature).
2. When ready to activate:
   - Produce in-chat Technical Refinement Block (per `docs/ORCHESTRATOR_PROTOCOL.md`).
   - Fill `docs/refinements/{WP_ID}.md` from template and run refinement validation.
   - Record refinement: `just record-refinement {WP_ID}` (writes `docs/ORCHESTRATOR_GATES.json`).
   - In a new user turn, after explicit approval evidence exists in the refinement file:
     - Record signature: `just record-signature {WP_ID} {usernameDDMMYYYYHHMM}`
       - Updates refinement file with APPROVED status and signature
       - Appends the signature to `docs/SIGNATURE_AUDIT.md`
       - Writes the signature event to `docs/ORCHESTRATOR_GATES.json`
   - Create WP worktree/branch: `just worktree-add {WP_ID}` (creates `feat/{WP_ID}` worktree)
   - Record prepare: `just record-prepare {WP_ID} {Coder-A|Coder-B} [branch] [worktree_dir]`
   - Create the official packet: `just create-task-packet {WP_ID}`
     - Script hard-gates on signature + prepare being recorded and on ENRICHMENT_NEEDED=NO.
3. Update `docs/TASK_BOARD.md` to move the item from STUB backlog to Ready for Dev.

Why:
- Prevents "packet by momentum" and ensures packet activation is both human-approved and mechanically logged.

### 4.3 Coder lifecycle: Ready for Dev -> In Progress -> Handoff
1. Coder claims the packet by setting:
   - `**Status:** In Progress`
   - `CODER_MODEL`
   - `CODER_REASONING_STRENGTH`
   (Enforced by `scripts/validation/task-packet-claim-check.mjs`)
2. Gate check: `just gate-check {WP_ID}` enforces Markdown phase ordering and "SKELETON APPROVED" before implementation signals.
3. Pre-work gate: `just pre-work {WP_ID}`
   - Validates packet structure
   - Validates refinement exists + signature matches
   - Enforces checkpoint commit gate for packet + refinement (prevents artifact loss)
   - Ensures deterministic manifest template exists
4. Coder implements within IN_SCOPE_PATHS and keeps evidence in the packet.
5. Post-work gate: `just post-work {WP_ID}`
   - Enforces deterministic manifest correctness (hashes, window bounds, line delta, path canonicalization).
   - Performs staged-aware checks to reduce false failures from unrelated local changes.

### 4.4 Validator lifecycle: audit -> PASS/FAIL -> commit gate
Validator uses both:
- Manual evidence audit (open files, map to file:line, re-run tests as needed)
- Mechanical validator scripts (scan, traceability, error-codes, DAL audit, git hygiene, etc.)

Additionally, Validator uses a mechanical gate sequence (writes to `docs/VALIDATOR_GATES.json`):
1. `just validator-gate-present {WP_ID} {PASS|FAIL}`
2. (After user acknowledgment) `just validator-gate-acknowledge {WP_ID}`
3. Append report to packet: `just validator-gate-append {WP_ID}`
4. If PASS, unlock commit: `just validator-gate-commit {WP_ID}`

Why: ensures the user sees the report before it is appended and before a commit is allowed.

### 4.5 Command-to-script mapping (what runs, what it reads/writes)

This table is intentionally explicit because these commands are the "mechanical glue" of the workflow.

| Command | Implementation | Reads | Writes |
|---|---|---|---|
| `just record-refinement {WP_ID}` | `scripts/validation/orchestrator_gates.mjs refine` | refinement file, `docs/SPEC_CURRENT.md` (+ spec file) | `docs/ORCHESTRATOR_GATES.json` |
| `just record-signature {WP_ID} {sig}` | `scripts/validation/orchestrator_gates.mjs sign` | `docs/ORCHESTRATOR_GATES.json`, refinement file, `docs/SIGNATURE_AUDIT.md`, repo grep for one-time signature | refinement file (sets APPROVED + signature), `docs/SIGNATURE_AUDIT.md`, `docs/ORCHESTRATOR_GATES.json` |
| `just worktree-add {WP_ID}` | `scripts/worktree-add.mjs` | git refs/worktree list | creates branch/worktree dir on disk (git operation) |
| `just record-prepare {WP_ID} ...` | `scripts/validation/orchestrator_gates.mjs prepare` | `docs/ORCHESTRATOR_GATES.json`, git branch exists, `git worktree list --porcelain` | `docs/ORCHESTRATOR_GATES.json` |
| `just create-task-packet {WP_ID}` | `scripts/create-task-packet.mjs` | refinement file, `docs/ORCHESTRATOR_GATES.json`, `docs/SIGNATURE_AUDIT.md`, `docs/templates/TASK_PACKET_TEMPLATE.md`, `docs/SPEC_CURRENT.md` | `docs/task_packets/{WP_ID}.md` (or creates refinement scaffold and exits BLOCKED) |
| `just gate-check {WP_ID}` | `scripts/validation/gate-check.mjs` | `docs/task_packets/{WP_ID}.md` | none |
| `just pre-work {WP_ID}` | `gate-check` + `scripts/validation/pre-work-check.mjs` | packet + refinement + signature audit; `scripts/validation/cor701-spec.json`; git object DB for checkpoint commit gate | may create `docs/task_packets/` dir if missing |
| `just post-work {WP_ID}` | `gate-check` + `scripts/validation/post-work-check.mjs` | packet; git diff/index/worktree files; spec schema `cor701-spec.json` | none |
| `just cor701-sha <file>` | `scripts/validation/cor701-sha.mjs` | git blobs (HEAD/INDEX) + worktree file | none |
| `just task-board-check` | `scripts/validation/task-board-check.mjs` | `docs/TASK_BOARD.md` | none |
| `just task-packet-claim-check` | `scripts/validation/task-packet-claim-check.mjs` | `docs/task_packets/*.md` | none |
| `just validator-gate-*` | `scripts/validation/validator_gates.mjs` | `docs/VALIDATOR_GATES.json`, (append gate checks packet exists) | `docs/VALIDATOR_GATES.json` |

Momentum/anti-bypass notes (current implementation):
- Orchestrator signature recording blocks if signature is recorded too soon after refinement (anti-momentum timer) and if USER_APPROVAL_EVIDENCE is missing/mismatched.
- Validator gates block if the next gate is executed within a minimum interval (anti-momentum).

---

## 5) Deterministic Manifest (COR-701 discipline)

Task packets contain a required `## VALIDATION` manifest block (template-enforced) with:
- target_file
- start/end line window
- line_delta
- pre_sha1 / post_sha1
- gates checklist

Key implementing components:
- Spec schema: `scripts/validation/cor701-spec.json`
- SHA helper: `scripts/validation/cor701-sha.mjs`
- Enforcement: `scripts/validation/post-work-check.mjs`

Notable implementation detail:
- `post-work-check.mjs` normalizes LF/CRLF variants for SHA convenience and uses staged diffs when possible to reduce cross-platform false negatives.

Why:
- Enables deterministic audits and "what exactly changed" provenance without trusting narrative summaries.

---

## 6) Concurrency and Worktrees

Core policy:
- One WP = one feature branch (`feat/{WP_ID}`) and (when >1 concurrent WP) one separate worktree per active WP.

Implementations:
- `docs/ROLE_WORKTREES.md`: defines role default worktrees/branches locally.
- `scripts/worktree-add.mjs` + `just worktree-add`: creates WP worktree/branch.
- `scripts/validation/worktree-concurrency-check.mjs`: local-only check; requires linked worktrees when multiple WPs are in progress.

Why:
- Prevents unstaged changes from one WP contaminating another WP's deterministic manifest/hygiene checks.

---

## 7) Context Management for Small-Context Models (Project-agnostic kernel)

The governance system is explicitly designed to support fresh models with small context windows by "front-loading" the needed context into machine-checkable artifacts.

### 7.1 How to decompose large work safely

Rule of thumb:
- If a task cannot be fully specified (scope, acceptance, tests, risks) in a single task packet without vague language, it should be split into multiple WPs.

Recommended decomposition strategies:
1. Split by invariant surface area:
   - Example: "migration idempotency" vs "down migrations" vs "test harness".
2. Split by layer boundary:
   - frontend UI vs backend API vs storage vs scripts.
3. Split by risk tier:
   - isolate HIGH-risk changes into their own WP so they can be audited independently.

### 7.2 How context is carried across sub-tasks

Carry context through artifacts, not chat memory:
- `docs/refinements/{WP_ID}.md` binds the packet to a specific spec version (sha1) and provides excerpt windows for anchors.
- Task packets embed:
  - exact IN_SCOPE_PATHS
  - DONE_MEANS
  - TEST_PLAN (copy-paste commands)
  - BOOTSTRAP (files to open, search terms, commands, risk map)
  - deterministic manifest(s)

This allows a new model to pick up work by reading:
- `docs/START_HERE.md`
- `docs/SPEC_CURRENT.md`
- the task packet
- the refinement

### 7.3 Model selection: when "heavy reasoning" is needed

This workflow reduces the need for large-context "hero" models by making work deterministic and decomposable. Heavy reasoning models still help when:
- The Master Spec slice is large and ambiguous.
- The work requires multi-layer architectural reasoning with high risk (security/storage).
- The required evidence mapping is extensive.

Otherwise, a standard model can execute WPs reliably when the packet and refinement are complete and the gates are passing.

---

## 8) Known Drift / Inconsistencies (Current repo state)

The governance system contains explicit drift that should be addressed to keep determinism intact:

Codex version references:
- CI workflow `.github/workflows/ci.yml` contains strings and messaging referencing "Codex v0.8".
- `scripts/validation/ci-traceability-check.mjs` explicitly checks for `Handshake Codex v0.8.md` (but the repo root currently contains `Handshake Codex v1.4.md`).
- `scripts/hooks/pre-commit` messaging references "Codex v0.8".
- `docs/task_packets/README.md` links to `Handshake Codex v0.8` (stale).
- `docs/PAST_WORK_INDEX.md` references much older spec/codex versions (stale).

Why this matters:
- These are governance enforcement surfaces (CI + hooks). If they refer to non-existent files/versions, they either fail unnecessarily or mislead operators/models.

Recommended remediation approach:
- Treat governance enforcement drift as its own remediation WP (so scripts/CI can be updated by a Coder under a signed packet), or explicitly declare a compatibility shim file if intentional.

---

## 9) Full Inventory (Snapshot)

Generated from repo file listing; grouped by directory. This is the "no file left out" surface for governance-oriented files and scripts.

### 9.1 Top-level directories
- `.cargo/`
- `.claude/`
- `.github/`
- `app/`
- `docs/`
- `docs_local/`
- `scripts/`
- `src/`
- `tests/`

### 9.2 Top-level files (repo root)
- `.codex_tmp_file`
- `.git`
- `.gitattributes`
- `.gitignore`
- `AGENTS.md`
- `deny.toml`
- `docker-compose.test.yml`
- `extraction and digital production team.md`
- `Handshake Codex v1.4.md`
- `Handshake_Export_Bundles_Insert_Plan_v0.1.md`
- `Handshake_logger_20251218.md`
- `Handshake_Master_Spec_v02.102.md`
- `Handshake_Master_Spec_v02.103.md`
- `Handshake_Master_Spec_v02.104.md`
- `Handshake_Master_Spec_v02.105.md`
- `Handshake_Master_Spec_v02.106.md`
- `Handshake_Master_Spec_v02.107.md`
- `Handshake_Phase_0_5_Closure_v0.1.md`
- `justfile`
- `n8n and feature mixing.md`
- `primitives_catalogue.md`
- `README.md`
- `STORAGE_PORTABILITY_ARCHITECTURE_GAP_ANALYSIS.md`
- `validation audit.md`

### 9.3 `.github/`
- `.github/workflows/ci.yml`

### 9.4 `.claude/`
- `.claude/settings.local.json`

### 9.5 `.cargo/`
- `.cargo/config.toml`

### 9.6 `scripts/`
- `scripts/README.md`
- `scripts/close-wp-branch.mjs`
- `scripts/codex-check-test.mjs`
- `scripts/create-task-packet.mjs`
- `scripts/new-api-endpoint.mjs`
- `scripts/new-react-component.mjs`
- `scripts/scaffold-check.mjs`
- `scripts/spec-current-check.mjs`
- `scripts/worktree-add.mjs`
- `scripts/fixtures/forbidden_fetch.ts`
- `scripts/fixtures/forbidden_todo.txt`
- `scripts/hooks/pre-commit`
- `scripts/validation/ci-traceability-check.mjs`
- `scripts/validation/codex-check.mjs`
- `scripts/validation/cor701-sha.mjs`
- `scripts/validation/cor701-spec.json`
- `scripts/validation/gate-check.mjs`
- `scripts/validation/orchestrator_gates.mjs`
- `scripts/validation/post-work-check.mjs`
- `scripts/validation/pre-work-check.mjs`
- `scripts/validation/refinement-check.mjs`
- `scripts/validation/task-board-check.mjs`
- `scripts/validation/task-packet-claim-check.mjs`
- `scripts/validation/validator_gates.mjs`
- `scripts/validation/validator-coverage-gaps.mjs`
- `scripts/validation/validator-dal-audit.mjs`
- `scripts/validation/validator-error-codes.mjs`
- `scripts/validation/validator-git-hygiene.mjs`
- `scripts/validation/validator-hygiene-full.mjs`
- `scripts/validation/validator-packet-complete.mjs`
- `scripts/validation/validator-phase-gate.mjs`
- `scripts/validation/validator-scan.mjs`
- `scripts/validation/validator-spec-regression.mjs`
- `scripts/validation/validator-traceability.mjs`
- `scripts/validation/worktree-concurrency-check.mjs`

### 9.7 `docs/`
- `docs/GOV_KERNEL/README.md`
- `docs/GOV_KERNEL/01_AUTHORITY_AND_ROLES.md`
- `docs/GOV_KERNEL/02_ARTIFACTS_AND_CONTRACTS.md`
- `docs/GOV_KERNEL/03_GATES_AND_ENFORCERS.md`
- `docs/GOV_KERNEL/04_SMALL_CONTEXT_HANDOFF.md`
- `docs/GOV_KERNEL/05_CI_HOOKS_AND_CONFIG.md`
- `docs/GOV_KERNEL/06_VERSIONING_AND_DRIFT_CONTROL.md`
- `docs/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md`
- `docs/AI_WORKFLOW_TEMPLATE.md` (compat shim; canonical under `docs/templates/`)
- `docs/ARCHITECTURE.md`
- `docs/CODER_IMPLEMENTATION_ROADMAP.md`
- `docs/CODER_PROTOCOL.md`
- `docs/CODER_PROTOCOL_GAPS.md`
- `docs/CODER_PROTOCOL_SCRUTINY.md`
- `docs/CODER_RUBRIC.md`
- `docs/MASTER_SPEC_INTENT_AUDIT_v02.103.md`
- `docs/MASTER_SPEC_MVP_ROADMAP_AUDIT_v02.103.md`
- `docs/MASTER_SPEC_SECTION_DIGEST_v02.103.md`
- `docs/MIGRATION_GUIDE.md`
- `docs/ORCHESTRATOR_GATES.json`
- `docs/ORCHESTRATOR_IMPLEMENTATION_ROADMAP.md`
- `docs/ORCHESTRATOR_PRIORITIES.md`
- `docs/ORCHESTRATOR_PROTOCOL.md`
- `docs/ORCHESTRATOR_PROTOCOL_GAPS.md`
- `docs/ORCHESTRATOR_RUBRIC.md`
- `docs/OSS_REGISTER.md`
- `docs/OWNERSHIP.md`
- `docs/PAST_WORK_INDEX.md`
- `docs/PHASE_1_EVIDENCE_MAP_v02.103.md`
- `docs/QUALITY_GATE.md`
- `docs/REFINEMENT_TEMPLATE.md` (compat shim; canonical under `docs/templates/`)
- `docs/ROADMAP_SECTION_COVERAGE_MATRIX_v02.103.md`
- `docs/ROADMAP_SECTION_COVERAGE_MATRIX_v02.107.md`
- `docs/ROADMAP_VS_MASTER_SPEC_AUDIT_v02.102.md`
- `docs/ROLE_WORKTREES.md`
- `docs/RUNBOOK_DEBUG.md`
- `docs/SIGNATURE_AUDIT.md`
- `docs/SPEC_CURRENT.md`
- `docs/START_HERE.md`
- `docs/TASK_BOARD.md`
- `docs/TASK_PACKET_TEMPLATE.md` (compat shim; canonical under `docs/templates/`)
- `docs/VALIDATOR_GATES.json`
- `docs/VALIDATOR_PROTOCOL.md`
- `docs/WP_TRACEABILITY_REGISTRY.md`
- `docs/workflow_technical_paper.md`
- `docs/adr/ADR-0001-handshake-architecture-and-governance.md`
- `docs/agents/AGENT_REGISTRY.md`
- `docs/messages history/coder claude code.md`
- `docs/messages history/coder gemini.md`
- `docs/messages history/coder gpt codex.md`
- `docs/messages history/orchestrator.md`
- `docs/messages history/Validator.md`
- `docs/Papers/HANDSHAKE_VISION_SYNTHESIS.md`
- `docs/refinements/README.md`
- `docs/refinements/WP-1-ACE-Validators-v4.md`
- `docs/refinements/WP-1-AppState-Refactoring-v3.md`
- `docs/refinements/WP-1-Dual-Backend-Tests-v2.md`
- `docs/refinements/WP-1-Flight-Recorder-v3.md`
- `docs/refinements/WP-1-LLM-Core-v3.md`
- `docs/refinements/WP-1-MEX-v1.2-Runtime-v3.md`
- `docs/refinements/WP-1-Migration-Framework-v2.md`
- `docs/refinements/WP-1-Operator-Consoles-v3.md`
- `docs/refinements/WP-1-OSS-Register-Enforcement-v1.md`
- `docs/refinements/WP-1-Spec-Enrichment-LLM-Core-v1.md`
- `docs/refinements/WP-1-Storage-Abstraction-Layer-v3.md`
- `docs/refinements/WP-1-Terminal-LAW-v3.md`
- `docs/refinements/WP-1-Tokenization-Service-v3.md`
- `docs/task_packets/README.md`
- `docs/task_packets/WP-1-ACE-Auditability.md`
- `docs/task_packets/WP-1-ACE-RAG-Plumbing.md`
- `docs/task_packets/WP-1-ACE-Runtime.md`
- `docs/task_packets/WP-1-ACE-Validators-v2.md`
- `docs/task_packets/WP-1-ACE-Validators-v3.md`
- `docs/task_packets/WP-1-ACE-Validators-v4.md`
- `docs/task_packets/WP-1-ACE-Validators.md`
- `docs/task_packets/WP-1-AI-Integration-Baseline.md`
- `docs/task_packets/WP-1-AI-Job-Model-v2.md`
- `docs/task_packets/WP-1-AI-Job-Model-v3.md`
- `docs/task_packets/WP-1-AI-Job-Model.md`
- `docs/task_packets/WP-1-AI-UX-Actions.md`
- `docs/task_packets/WP-1-AI-UX-Rewrite.md`
- `docs/task_packets/WP-1-AI-UX-Summarize-Display.md`
- `docs/task_packets/WP-1-AppState-Refactoring-v2.md`
- `docs/task_packets/WP-1-AppState-Refactoring-v3.md`
- `docs/task_packets/WP-1-AppState-Refactoring.md`
- `docs/task_packets/WP-1-Atelier-Lens-v0.1.md`
- `docs/task_packets/WP-1-Atelier-Lens.md`
- `docs/task_packets/WP-1-Bundle-Export.md`
- `docs/task_packets/WP-1-Calendar-Lens.md`
- `docs/task_packets/WP-1-Canvas-Typography.md`
- `docs/task_packets/WP-1-Capability-Enforcement.md`
- `docs/task_packets/WP-1-Capability-SSoT-Validator.md`
- `docs/task_packets/WP-1-Capability-SSoT.md`
- `docs/task_packets/WP-1-Debug-Bundle-v2.md`
- `docs/task_packets/WP-1-Debug-Bundle-v3.md`
- `docs/task_packets/WP-1-Debug-Bundle.md`
- `docs/task_packets/WP-1-Diagnostic-Pipe.md`
- `docs/task_packets/WP-1-Distillation-Logging.md`
- `docs/task_packets/WP-1-Distillation.md`
- `docs/task_packets/WP-1-Dual-Backend-Tests-v2.md`
- `docs/task_packets/WP-1-Dual-Backend-Tests.md`
- `docs/task_packets/WP-1-Editor-Hardening.md`
- `docs/task_packets/WP-1-Flight-Recorder-UI-v2.md`
- `docs/task_packets/WP-1-Flight-Recorder-UI.md`
- `docs/task_packets/WP-1-Flight-Recorder-v2.md`
- `docs/task_packets/WP-1-Flight-Recorder-v3.md`
- `docs/task_packets/WP-1-Flight-Recorder.md`
- `docs/task_packets/WP-1-Frontend-AI-Action.md`
- `docs/task_packets/WP-1-Frontend-Build-Debug.md`
- `docs/task_packets/WP-1-Gate-Check-Tool-v2.md`
- `docs/task_packets/WP-1-Gate-Check-Tool.md`
- `docs/task_packets/WP-1-Governance-Hooks.md`
- `docs/task_packets/WP-1-LLM-Core-v3.md`
- `docs/task_packets/WP-1-LLM-Core.md`
- `docs/task_packets/WP-1-MCP-End-to-End.md`
- `docs/task_packets/WP-1-MCP-Skeleton-Gate.md`
- `docs/task_packets/WP-1-Mechanical-Track-Full.md`
- `docs/task_packets/WP-1-Metrics-OTel.md`
- `docs/task_packets/WP-1-Metrics-Traces.md`
- `docs/task_packets/WP-1-MEX-Observability.md`
- `docs/task_packets/WP-1-MEX-Safety-Gates.md`
- `docs/task_packets/WP-1-MEX-UX-Bridges.md`
- `docs/task_packets/WP-1-MEX-v1.2-Runtime-v2.md`
- `docs/task_packets/WP-1-MEX-v1.2-Runtime-v3.md`
- `docs/task_packets/WP-1-MEX-v1.2-Runtime.md`
- `docs/task_packets/WP-1-Migration-Framework.md`
- `docs/task_packets/WP-1-Model-Profiles.md`
- `docs/task_packets/WP-1-Mutation-Traceability.md`
- `docs/task_packets/WP-1-Operator-Consoles-v1.md`
- `docs/task_packets/WP-1-Operator-Consoles-v2.md`
- `docs/task_packets/WP-1-Operator-Consoles-v3.md`
- `docs/task_packets/WP-1-Operator-Consoles.md`
- `docs/task_packets/WP-1-OSS-Governance.md`
- `docs/task_packets/WP-1-OSS-Register-Enforcement-v1.md`
- `docs/task_packets/WP-1-PDF-Pipeline.md`
- `docs/task_packets/WP-1-Photo-Studio-Skeleton.md`
- `docs/task_packets/WP-1-Photo-Studio.md`
- `docs/task_packets/WP-1-RAG-Iterative.md`
- `docs/task_packets/WP-1-Retention-GC.md`
- `docs/task_packets/WP-1-Security-Gates-v2.md`
- `docs/task_packets/WP-1-Security-Gates-v3.md`
- `docs/task_packets/WP-1-Security-Gates.md`
- `docs/task_packets/WP-1-Semantic-Catalog.md`
- `docs/task_packets/WP-1-Spec-Enrichment-LLM-Core-v1.md`
- `docs/task_packets/WP-1-Storage-Abstraction-Layer-v2.md`
- `docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md`
- `docs/task_packets/WP-1-Storage-Abstraction-Layer.md`
- `docs/task_packets/WP-1-Storage-Foundation-20251228.md`
- `docs/task_packets/WP-1-Storage-Foundation-v3.md`
- `docs/task_packets/WP-1-Storage-Foundation.md`
- `docs/task_packets/WP-1-Supply-Chain-MEX.md`
- `docs/task_packets/WP-1-Terminal-Integration-Baseline.md`
- `docs/task_packets/WP-1-Terminal-LAW-v2.md`
- `docs/task_packets/WP-1-Terminal-LAW-v3.md`
- `docs/task_packets/WP-1-Terminal-LAW.md`
- `docs/task_packets/WP-1-Tokenization-Service-20251228.md`
- `docs/task_packets/WP-1-Tokenization-Service-v3.md`
- `docs/task_packets/WP-1-Tokenization-Service.md`
- `docs/task_packets/WP-1-Validator-Error-Codes-v1.md`
- `docs/task_packets/WP-1-Workflow-Engine-v2.md`
- `docs/task_packets/WP-1-Workflow-Engine-v3.md`
- `docs/task_packets/WP-1-Workflow-Engine-v4.md`
- `docs/task_packets/WP-1-Workflow-Engine.md`
- `docs/task_packets/WP-1-Workspace-Bundle.md`
- `docs/task_packets/stubs/README.md`
- `docs/task_packets/stubs/WP-1-ACE-Auditability-v2.md`
- `docs/task_packets/stubs/WP-1-ACE-Runtime-v2.md`
- `docs/task_packets/stubs/WP-1-AI-Job-Model-v4.md`
- `docs/task_packets/stubs/WP-1-AI-UX-Actions-v2.md`
- `docs/task_packets/stubs/WP-1-AI-UX-Rewrite-v2.md`
- `docs/task_packets/stubs/WP-1-AI-UX-Summarize-Display-v2.md`
- `docs/task_packets/stubs/WP-1-Atelier-Lens-v2.md`
- `docs/task_packets/stubs/WP-1-Calendar-Lens-v2.md`
- `docs/task_packets/stubs/WP-1-Canvas-Typography-v2.md`
- `docs/task_packets/stubs/WP-1-Capability-SSoT-v2.md`
- `docs/task_packets/stubs/WP-1-Cross-Tool-Interaction-Conformance-v1.md`
- `docs/task_packets/stubs/WP-1-Dev-Experience-ADRs.md`
- `docs/task_packets/stubs/WP-1-Distillation-v2.md`
- `docs/task_packets/stubs/WP-1-Editor-Hardening-v2.md`
- `docs/task_packets/stubs/WP-1-Flight-Recorder-UI-v3.md`
- `docs/task_packets/stubs/WP-1-Global-Silent-Edit-Guard.md`
- `docs/task_packets/stubs/WP-1-Governance-Kernel-Conformance-v1.md`
- `docs/task_packets/stubs/WP-1-Governance-Hooks-v2.md`
- `docs/task_packets/stubs/WP-1-LocalFirst-Agentic-MCP-Posture-v1.md`
- `docs/task_packets/stubs/WP-1-MCP-End-to-End-v2.md`
- `docs/task_packets/stubs/WP-1-MCP-Skeleton-Gate-v2.md`
- `docs/task_packets/stubs/WP-1-Metrics-OTel-v2.md`
- `docs/task_packets/stubs/WP-1-Metrics-Traces-v2.md`
- `docs/task_packets/stubs/WP-1-MEX-Observability-v2.md`
- `docs/task_packets/stubs/WP-1-MEX-Safety-Gates-v2.md`
- `docs/task_packets/stubs/WP-1-MEX-UX-Bridges-v2.md`
- `docs/task_packets/stubs/WP-1-Migration-Framework-v2.md`
- `docs/task_packets/stubs/WP-1-Model-Profiles-v2.md`
- `docs/task_packets/stubs/WP-1-Mutation-Traceability-v2.md`
- `docs/task_packets/stubs/WP-1-OSS-Governance-v2.md`
- `docs/task_packets/stubs/WP-1-PDF-Pipeline-v2.md`
- `docs/task_packets/stubs/WP-1-Photo-Studio-v2.md`
- `docs/task_packets/stubs/WP-1-RAG-Iterative-v2.md`
- `docs/task_packets/stubs/WP-1-Response-Behavior-ANS-001.md`
- `docs/task_packets/stubs/WP-1-Semantic-Catalog-v2.md`
- `docs/task_packets/stubs/WP-1-Spec-Router-Session-Log.md`
- `docs/task_packets/stubs/WP-1-Supply-Chain-MEX-v2.md`
- `docs/task_packets/stubs/WP-1-Workspace-Bundle-v2.md`
- `docs/templates/AI_WORKFLOW_TEMPLATE.md`
- `docs/templates/REFINEMENT_TEMPLATE.md`
- `docs/templates/TASK_PACKET_STUB_TEMPLATE.md`
- `docs/templates/TASK_PACKET_TEMPLATE.md`

### 9.8 `docs_local/`
- `docs_local/DOC_INDEX.txt`
- `docs_local/Diary RID extraction.txt`
- `docs_local/legacy/The_Prompt_Diaries_v03.056.000_2025-11-28_01-42_CET.txt`
