# Reference Implementation: Handshake Governance (Non-Normative)

Purpose: map a concrete repository (Handshake) to the project-agnostic governance kernel (`.GOV/GOV_KERNEL/*`), including an exhaustive inventory of governing artifacts and enforcement scripts.

Scope:
- This is a governance/operations spec, not product behavior.
- It documents (a) the kernel-level concepts and (b) how they are concretely implemented by files/scripts in this repo.
- "No file left out" means: every file under the governance surface (.GOV/, .GOV/operator/docs_local/, .github/) plus all root-level governance/config files are enumerated in the inventory section.

Non-goals:
- Do not change product code (`src/`, `app/`, `tests/`) here.
- Do not treat this document as a replacement for the authoritative law stack (Codex + Master Spec + protocols); it is an implementation map and inventory.

---

## 1) Authority Stack (LAW) and Precedence

The governance system is explicit about precedence. The current implemented stack is:

1. `Handshake Codex v1.4.md` (repo root)
   - Defines repo invariants and allowed assistant behavior (including hard bans on destructive ops and git worktree/branch rewrites without explicit consent).
2. Master Spec: `Handshake_Master_Spec_v*.md` (repo root), with pointer file `.GOV/roles_shared/SPEC_CURRENT.md`
   - Product intent + architecture + normative requirements; "Main Body first" discipline is enforced mechanically.
3. Protocol layer in `.GOV/`
   - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Orchestrator behavior and signature/refinement workflow)
   - `.GOV/roles/coder/CODER_PROTOCOL.md` (Coder behavior and phase gating)
   - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md` (Validator behavior and evidence-based audit rules)
4. Repo guardrails: `AGENTS.md`
   - Local hard rules specific to this repo (no destructive cleanup; WP branching/worktree; checkpoint commit gate).
5. Mechanical enforcement scripts + `justfile`
   - Deterministic checks and workflow gates implemented as executable scripts under `.GOV/scripts/` and invoked via `just`.

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
- Debugger (triage; uses `.GOV/roles_shared/RUNBOOK_DEBUG.md`)
- Tooling agent (runs scripts / builds diagnostic bundles)
- Red-team framing exists as a responsibility inside Validator and Refinement blocks.

---

## 3) Canonical Governance Artifacts (What exists, and why)

This section describes the key governance artifacts and how they gate each other.

### 3.1 Navigation + orientation pack (`.GOV/`)
- `.GOV/roles_shared/START_HERE.md`: canonical entry point and command surface.
- `.GOV/roles_shared/ARCHITECTURE.md`: module map and allowed dependency boundaries.
- `.GOV/roles_shared/RUNBOOK_DEBUG.md`: incident/CI triage, log locations, minimal debug flow.
- `.GOV/roles_shared/PAST_WORK_INDEX.md`: archaeology pointers (note: this file currently contains stale references; see Section 8).

Why: enables a fresh model (or human) to orient quickly and deterministically without reading the whole repo.

### 3.2 Spec pointer and spec drift guard
- `.GOV/roles_shared/SPEC_CURRENT.md`: the single pointer to the current authoritative Master Spec.
- `.GOV/scripts/spec-current-check.mjs`: enforces that SPEC_CURRENT points to the latest `Handshake_Master_Spec_v*.md` file by parsed version.

Why: prevents silent spec drift and "coding against an old spec".

### 3.3 Task Board (execution state SSoT)
- `.GOV/roles_shared/TASK_BOARD.md`: the global state tracker for Phase 1 WPs.
- `.GOV/scripts/validation/task-board-check.mjs`: enforces strict formatting for `## In Progress`, `## Done`, `## Superseded` entries.

Key rule (enforced in docs and protocols):
- Task Board is intentionally minimal; detailed reasons live in packets.

### 3.4 Work Packet Traceability (Base WP -> Active Packet mapping)
- `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`: resolves Base WP IDs to an Active Packet file path, especially when revisions exist (`-vN`).

Why: the Master Spec should not embed revision packet IDs; this registry prevents ambiguity when multiple packet revisions exist.

### 3.5 Stubs vs activated packets
- Stubs: `.GOV/task_packets/stubs/` (not executable)
- Activated packets: `.GOV/task_packets/` (executable authority for implementation/validation)
- Templates:
  - `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`

Why: allows backlog reshaping without consuming signatures, while keeping "Ready for Dev" meaningfully executable.

### 3.6 Refinements (Technical Refinement Block artifacts)
- `.GOV/refinements/{WP_ID}.md`: per-WP refinement artifact.
- Template: `.GOV/templates/REFINEMENT_TEMPLATE.md`
- Mechanical enforcement: `.GOV/scripts/validation/refinement-check.mjs`

Key implemented properties:
- ASCII-only.
- Includes SPEC_TARGET_RESOLVED + SPEC_TARGET_SHA1 binding to the current spec file.
- Includes SPEC_ANCHORS with excerpt window and token-in-window match requirements.
- Includes "CLEARLY_COVERS" 5-point checklist and ENRICHMENT decision.
- Includes `USER_APPROVAL_EVIDENCE` as a deterministic guard against momentum.

Why: makes spec anchoring checkable and portable across small-context models.

### 3.7 Signatures (one-time, auditable)
- `.GOV/roles_shared/SIGNATURE_AUDIT.md`: authoritative registry of signatures consumed.
- Orchestrator gate script appends entries and enforces one-time use.

Why: creates a forced alignment pause and prevents autonomous drift.

### 3.8 Gate state logs (machine-readable)
- `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`: log of REFINE/SIGN/PREPARE events.
- `.GOV/validator_gates/{WP_ID}.json`: log of validation gate sequence (present -> acknowledge -> append -> commit).
- (Legacy) `.GOV/roles/validator/VALIDATOR_GATES.json`: read-only archive of older validation sessions.

Why: provides deterministic, machine-checkable proof that the workflow was followed.

### 3.9 Quality gate definition
- `.GOV/roles_shared/QUALITY_GATE.md`: Gate 0 (pre-work) and Gate 1 (post-work) definitions; risk tier matrix; required commands.

Why: sets a minimum hygiene baseline; prevents "it compiled on my machine" merges.

### 3.10 Role-local worktree policy
- `.GOV/roles_shared/ROLE_WORKTREES.md`: local mapping of role -> (worktree dir, branch) on the operator machine.

Why: prevents role confusion and cross-WP contamination; makes "where am I?" checkable.

### 3.11 Ownership and agent identity
- `.GOV/roles_shared/OWNERSHIP.md`: area owners for review routing.
- `.GOV/agents/AGENT_REGISTRY.md`: agent IDs and role mapping.

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
1. Create stub file in `.GOV/task_packets/stubs/` (no signature).
2. When ready to activate:
   - Produce in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
   - Fill `.GOV/refinements/{WP_ID}.md` from template and run refinement validation.
   - Record refinement: `just record-refinement {WP_ID}` (writes `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`).
   - In a new user turn, after explicit approval evidence exists in the refinement file:
     - Record signature: `just record-signature {WP_ID} {usernameDDMMYYYYHHMM}`
       - Updates refinement file with APPROVED status and signature
       - Appends the signature to `.GOV/roles_shared/SIGNATURE_AUDIT.md`
       - Writes the signature event to `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`
   - Create WP worktree/branch: `just worktree-add {WP_ID}` (creates `feat/{WP_ID}` worktree)
   - Record prepare: `just record-prepare {WP_ID} {Coder-A|Coder-B} [branch] [worktree_dir]`
   - Create the official packet: `just create-task-packet {WP_ID}`
     - Script hard-gates on signature + prepare being recorded and on ENRICHMENT_NEEDED=NO.
3. Update `.GOV/roles_shared/TASK_BOARD.md` to move the item from STUB backlog to Ready for Dev.

Why:
- Prevents "packet by momentum" and ensures packet activation is both human-approved and mechanically logged.

### 4.3 Coder lifecycle: Ready for Dev -> In Progress -> Handoff
1. Coder claims the packet by setting:
   - `**Status:** In Progress`
   - `CODER_MODEL`
   - `CODER_REASONING_STRENGTH`
   (Enforced by `.GOV/scripts/validation/task-packet-claim-check.mjs`)
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

Additionally, Validator uses a mechanical gate sequence (writes per WP to `.GOV/validator_gates/{WP_ID}.json`):
1. `just validator-gate-present {WP_ID} {PASS|FAIL}`
2. (After user acknowledgment) `just validator-gate-acknowledge {WP_ID}`
3. Append report to packet: `just validator-gate-append {WP_ID}`
4. If PASS, unlock commit: `just validator-gate-commit {WP_ID}`

Why: ensures the user sees the report before it is appended and before a commit is allowed.

### 4.5 Command-to-script mapping (what runs, what it reads/writes)

This table is intentionally explicit because these commands are the "mechanical glue" of the workflow.

| Command | Implementation | Reads | Writes |
|---|---|---|---|
| `just record-refinement {WP_ID}` | `.GOV/scripts/validation/orchestrator_gates.mjs refine` | refinement file, `.GOV/roles_shared/SPEC_CURRENT.md` (+ spec file) | `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` |
| `just record-signature {WP_ID} {sig}` | `.GOV/scripts/validation/orchestrator_gates.mjs sign` | `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`, refinement file, `.GOV/roles_shared/SIGNATURE_AUDIT.md`, repo grep for one-time signature | refinement file (sets APPROVED + signature), `.GOV/roles_shared/SIGNATURE_AUDIT.md`, `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` |
| `just worktree-add {WP_ID}` | `.GOV/scripts/worktree-add.mjs` | git refs/worktree list | creates branch/worktree dir on disk (git operation) |
| `just record-prepare {WP_ID} ...` | `.GOV/scripts/validation/orchestrator_gates.mjs prepare` | `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`, git branch exists, `git worktree list --porcelain` | `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` |
| `just create-task-packet {WP_ID}` | `.GOV/scripts/create-task-packet.mjs` | refinement file, `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`, `.GOV/roles_shared/SIGNATURE_AUDIT.md`, `.GOV/templates/TASK_PACKET_TEMPLATE.md`, `.GOV/roles_shared/SPEC_CURRENT.md` | `.GOV/task_packets/{WP_ID}.md` (or creates refinement scaffold and exits BLOCKED) |
| `just gate-check {WP_ID}` | `.GOV/scripts/validation/gate-check.mjs` | `.GOV/task_packets/{WP_ID}.md` | none |
| `just pre-work {WP_ID}` | `gate-check` + `.GOV/scripts/validation/pre-work-check.mjs` | packet + refinement + signature audit; `.GOV/scripts/validation/cor701-spec.json`; git object DB for checkpoint commit gate | may create `.GOV/task_packets/` dir if missing |
| `just post-work {WP_ID}` | `gate-check` + `.GOV/scripts/validation/post-work-check.mjs` | packet; git diff/index/worktree files; spec schema `cor701-spec.json` | none |
| `just cor701-sha <file>` | `.GOV/scripts/validation/cor701-sha.mjs` | git blobs (HEAD/INDEX) + worktree file | none |
| `just task-board-check` | `.GOV/scripts/validation/task-board-check.mjs` | `.GOV/roles_shared/TASK_BOARD.md` | none |
| `just task-packet-claim-check` | `.GOV/scripts/validation/task-packet-claim-check.mjs` | `.GOV/task_packets/*.md` | none |
| `just validator-gate-*` | `.GOV/scripts/validation/validator_gates.mjs` | `.GOV/validator_gates/{WP_ID}.json` (or legacy `.GOV/roles/validator/VALIDATOR_GATES.json` for older sessions), (append gate checks packet exists) | `.GOV/validator_gates/{WP_ID}.json` |

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
- Spec schema: `.GOV/scripts/validation/cor701-spec.json`
- SHA helper: `.GOV/scripts/validation/cor701-sha.mjs`
- Enforcement: `.GOV/scripts/validation/post-work-check.mjs`

Notable implementation detail:
- `post-work-check.mjs` normalizes LF/CRLF variants for SHA convenience and uses staged diffs when possible to reduce cross-platform false negatives.

Why:
- Enables deterministic audits and "what exactly changed" provenance without trusting narrative summaries.

---

## 6) Concurrency and Worktrees

Core policy:
- One WP = one feature branch (`feat/{WP_ID}`) and (when >1 concurrent WP) one separate worktree per active WP.

Implementations:
- `.GOV/roles_shared/ROLE_WORKTREES.md`: defines role default worktrees/branches locally.
- `.GOV/scripts/worktree-add.mjs` + `just worktree-add`: creates WP worktree/branch.
- `.GOV/scripts/validation/worktree-concurrency-check.mjs`: local-only check; requires linked worktrees when multiple WPs are in progress.

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
- `.GOV/refinements/{WP_ID}.md` binds the packet to a specific spec version (sha1) and provides excerpt windows for anchors.
- Task packets embed:
  - exact IN_SCOPE_PATHS
  - DONE_MEANS
  - TEST_PLAN (copy-paste commands)
  - BOOTSTRAP (files to open, search terms, commands, risk map)
  - deterministic manifest(s)

This allows a new model to pick up work by reading:
- `.GOV/roles_shared/START_HERE.md`
- `.GOV/roles_shared/SPEC_CURRENT.md`
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
- `.GOV/scripts/validation/ci-traceability-check.mjs` explicitly checks for `Handshake Codex v0.8.md` (but the repo root currently contains `Handshake Codex v1.4.md`).
- `.GOV/scripts/hooks/pre-commit` messaging references "Codex v0.8".
- `.GOV/task_packets/README.md` links to `Handshake Codex v0.8` (stale).
- `.GOV/roles_shared/PAST_WORK_INDEX.md` references much older spec/codex versions (stale).

Why this matters:
- These are governance enforcement surfaces (CI + hooks). If they refer to non-existent files/versions, they either fail unnecessarily or mislead operators/models.

Recommended remediation approach:
- Treat governance enforcement drift as its own remediation WP (so .GOV/scripts/CI can be updated by a Coder under a signed packet), or explicitly declare a compatibility shim file if intentional.

---

## 9) Full Inventory (Snapshot)

Generated from repo file listing; grouped by directory. This is the "no file left out" surface for governance-oriented files and scripts.

### 9.1 Top-level directories
- `.cargo/`
- `.claude/`
- `.github/`
- `app/`
- `.GOV/`
- `.GOV/operator/docs_local/`
- `.GOV/scripts/`
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
- `.GOV/operator/extraction and digital production team.md`
- `Handshake Codex v1.4.md`
- `.GOV/operator/Handshake_Export_Bundles_Insert_Plan_v0.1.md`
- `Handshake_logger_20251218.md`
- `Handshake_Master_Spec_v02.102.md`
- `Handshake_Master_Spec_v02.103.md`
- `Handshake_Master_Spec_v02.104.md`
- `Handshake_Master_Spec_v02.105.md`
- `Handshake_Master_Spec_v02.106.md`
- `Handshake_Master_Spec_v02.107.md`
- `Handshake_Phase_0_5_Closure_v0.1.md`
- `justfile`
- `.GOV/operator/n8n and feature mixing.md`
- `.GOV/operator/primitives_catalogue.md`
- `README.md`
- `.GOV/operator/STORAGE_PORTABILITY_ARCHITECTURE_GAP_ANALYSIS.md`
- `.GOV/operator/validation audit.md`

### 9.3 `.github/`
- `.github/workflows/ci.yml`

### 9.4 `.claude/`
- `.claude/settings.local.json`

### 9.5 `.cargo/`
- `.cargo/config.toml`

### 9.6 `.GOV/scripts/`
- `.GOV/scripts/README.md`
- `.GOV/scripts/close-wp-branch.mjs`
- `.GOV/scripts/codex-check-test.mjs`
- `.GOV/scripts/create-task-packet.mjs`
- `.GOV/scripts/new-api-endpoint.mjs`
- `.GOV/scripts/new-react-component.mjs`
- `.GOV/scripts/scaffold-check.mjs`
- `.GOV/scripts/spec-current-check.mjs`
- `.GOV/scripts/worktree-add.mjs`
- `.GOV/scripts/fixtures/forbidden_fetch.ts`
- `.GOV/scripts/fixtures/forbidden_todo.txt`
- `.GOV/scripts/hooks/pre-commit`
- `.GOV/scripts/validation/ci-traceability-check.mjs`
- `.GOV/scripts/validation/codex-check.mjs`
- `.GOV/scripts/validation/cor701-sha.mjs`
- `.GOV/scripts/validation/cor701-spec.json`
- `.GOV/scripts/validation/gate-check.mjs`
- `.GOV/scripts/validation/orchestrator_gates.mjs`
- `.GOV/scripts/validation/post-work-check.mjs`
- `.GOV/scripts/validation/pre-work-check.mjs`
- `.GOV/scripts/validation/refinement-check.mjs`
- `.GOV/scripts/validation/task-board-check.mjs`
- `.GOV/scripts/validation/task-packet-claim-check.mjs`
- `.GOV/scripts/validation/validator_gates.mjs`
- `.GOV/scripts/validation/validator-coverage-gaps.mjs`
- `.GOV/scripts/validation/validator-dal-audit.mjs`
- `.GOV/scripts/validation/validator-error-codes.mjs`
- `.GOV/scripts/validation/validator-git-hygiene.mjs`
- `.GOV/scripts/validation/validator-hygiene-full.mjs`
- `.GOV/scripts/validation/validator-packet-complete.mjs`
- `.GOV/scripts/validation/validator-phase-gate.mjs`
- `.GOV/scripts/validation/validator-scan.mjs`
- `.GOV/scripts/validation/validator-spec-regression.mjs`
- `.GOV/scripts/validation/validator-traceability.mjs`
- `.GOV/scripts/validation/worktree-concurrency-check.mjs`

### 9.7 `.GOV/`
- `.GOV/GOV_KERNEL/README.md`
- `.GOV/GOV_KERNEL/01_AUTHORITY_AND_ROLES.md`
- `.GOV/GOV_KERNEL/02_ARTIFACTS_AND_CONTRACTS.md`
- `.GOV/GOV_KERNEL/03_GATES_AND_ENFORCERS.md`
- `.GOV/GOV_KERNEL/04_SMALL_CONTEXT_HANDOFF.md`
- `.GOV/GOV_KERNEL/05_CI_HOOKS_AND_CONFIG.md`
- `.GOV/GOV_KERNEL/06_VERSIONING_AND_DRIFT_CONTROL.md`
- `.GOV/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md`
- `.GOV/templates/AI_WORKFLOW_TEMPLATE.md` (compat shim; canonical under `.GOV/templates/`)
- `.GOV/roles_shared/ARCHITECTURE.md`
- `.GOV/roles/coder/CODER_IMPLEMENTATION_ROADMAP.md`
- `.GOV/roles/coder/CODER_PROTOCOL.md`
- `.GOV/roles/coder/CODER_PROTOCOL_GAPS.md`
- `.GOV/roles/coder/CODER_PROTOCOL_SCRUTINY.md`
- `.GOV/roles/coder/CODER_RUBRIC.md`
- `.GOV/roles_shared/MASTER_SPEC_INTENT_AUDIT_v02.103.md`
- `.GOV/roles_shared/MASTER_SPEC_MVP_ROADMAP_AUDIT_v02.103.md`
- `.GOV/roles_shared/MASTER_SPEC_SECTION_DIGEST_v02.103.md`
- `.GOV/roles_shared/MIGRATION_GUIDE.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`
- `.GOV/roles/orchestrator/ORCHESTRATOR_IMPLEMENTATION_ROADMAP.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_PRIORITIES.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL_GAPS.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_RUBRIC.md`
- `.GOV/roles_shared/OSS_REGISTER.md`
- `.GOV/roles_shared/OWNERSHIP.md`
- `.GOV/roles_shared/PAST_WORK_INDEX.md`
- `.GOV/roles_shared/PHASE_1_EVIDENCE_MAP_v02.103.md`
- `.GOV/roles_shared/QUALITY_GATE.md`
- `.GOV/templates/REFINEMENT_TEMPLATE.md` (compat shim; canonical under `.GOV/templates/`)
- `.GOV/roles_shared/ROADMAP_SECTION_COVERAGE_MATRIX_v02.103.md`
- `.GOV/roles_shared/ROADMAP_SECTION_COVERAGE_MATRIX_v02.107.md`
- `.GOV/roles_shared/ROADMAP_VS_MASTER_SPEC_AUDIT_v02.102.md`
- `.GOV/roles_shared/ROLE_WORKTREES.md`
- `.GOV/roles_shared/RUNBOOK_DEBUG.md`
- `.GOV/roles_shared/SIGNATURE_AUDIT.md`
- `.GOV/roles_shared/SPEC_CURRENT.md`
- `.GOV/roles_shared/START_HERE.md`
- `.GOV/roles_shared/TASK_BOARD.md`
- `.GOV/templates/TASK_PACKET_TEMPLATE.md` (compat shim; canonical under `.GOV/templates/`)
- `.GOV/validator_gates/{WP_ID}.json`
- (Legacy) `.GOV/roles/validator/VALIDATOR_GATES.json`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`
- `.GOV/roles_shared/workflow_technical_paper.md`
- `.GOV/adr/ADR-0001-handshake-architecture-and-governance.md`
- `.GOV/agents/AGENT_REGISTRY.md`
- `.GOV/messages history/coder claude code.md`
- `.GOV/messages history/coder gemini.md`
- `.GOV/messages history/coder gpt codex.md`
- `.GOV/messages history/orchestrator.md`
- `.GOV/messages history/Validator.md`
- `.GOV/Papers/HANDSHAKE_VISION_SYNTHESIS.md`
- `.GOV/refinements/README.md`
- `.GOV/refinements/WP-1-ACE-Validators-v4.md`
- `.GOV/refinements/WP-1-AppState-Refactoring-v3.md`
- `.GOV/refinements/WP-1-Dual-Backend-Tests-v2.md`
- `.GOV/refinements/WP-1-Flight-Recorder-v3.md`
- `.GOV/refinements/WP-1-LLM-Core-v3.md`
- `.GOV/refinements/WP-1-MEX-v1.2-Runtime-v3.md`
- `.GOV/refinements/WP-1-Migration-Framework-v2.md`
- `.GOV/refinements/WP-1-Operator-Consoles-v3.md`
- `.GOV/refinements/WP-1-OSS-Register-Enforcement-v1.md`
- `.GOV/refinements/WP-1-Spec-Enrichment-LLM-Core-v1.md`
- `.GOV/refinements/WP-1-Storage-Abstraction-Layer-v3.md`
- `.GOV/refinements/WP-1-Terminal-LAW-v3.md`
- `.GOV/refinements/WP-1-Tokenization-Service-v3.md`
- `.GOV/task_packets/README.md`
- `.GOV/task_packets/WP-1-ACE-Auditability.md`
- `.GOV/task_packets/WP-1-ACE-RAG-Plumbing.md`
- `.GOV/task_packets/WP-1-ACE-Runtime.md`
- `.GOV/task_packets/WP-1-ACE-Validators-v2.md`
- `.GOV/task_packets/WP-1-ACE-Validators-v3.md`
- `.GOV/task_packets/WP-1-ACE-Validators-v4.md`
- `.GOV/task_packets/WP-1-ACE-Validators.md`
- `.GOV/task_packets/WP-1-AI-Integration-Baseline.md`
- `.GOV/task_packets/WP-1-AI-Job-Model-v2.md`
- `.GOV/task_packets/WP-1-AI-Job-Model-v3.md`
- `.GOV/task_packets/WP-1-AI-Job-Model.md`
- `.GOV/task_packets/WP-1-AI-UX-Actions.md`
- `.GOV/task_packets/WP-1-AI-UX-Rewrite.md`
- `.GOV/task_packets/WP-1-AI-UX-Summarize-Display.md`
- `.GOV/task_packets/WP-1-AppState-Refactoring-v2.md`
- `.GOV/task_packets/WP-1-AppState-Refactoring-v3.md`
- `.GOV/task_packets/WP-1-AppState-Refactoring.md`
- `.GOV/task_packets/WP-1-Atelier-Lens-v0.1.md`
- `.GOV/task_packets/WP-1-Atelier-Lens.md`
- `.GOV/task_packets/WP-1-Bundle-Export.md`
- `.GOV/task_packets/WP-1-Calendar-Lens.md`
- `.GOV/task_packets/WP-1-Canvas-Typography.md`
- `.GOV/task_packets/WP-1-Capability-Enforcement.md`
- `.GOV/task_packets/WP-1-Capability-SSoT-Validator.md`
- `.GOV/task_packets/WP-1-Capability-SSoT.md`
- `.GOV/task_packets/WP-1-Debug-Bundle-v2.md`
- `.GOV/task_packets/WP-1-Debug-Bundle-v3.md`
- `.GOV/task_packets/WP-1-Debug-Bundle.md`
- `.GOV/task_packets/WP-1-Diagnostic-Pipe.md`
- `.GOV/task_packets/WP-1-Distillation-Logging.md`
- `.GOV/task_packets/WP-1-Distillation.md`
- `.GOV/task_packets/WP-1-Dual-Backend-Tests-v2.md`
- `.GOV/task_packets/WP-1-Dual-Backend-Tests.md`
- `.GOV/task_packets/WP-1-Editor-Hardening.md`
- `.GOV/task_packets/WP-1-Flight-Recorder-UI-v2.md`
- `.GOV/task_packets/WP-1-Flight-Recorder-UI.md`
- `.GOV/task_packets/WP-1-Flight-Recorder-v2.md`
- `.GOV/task_packets/WP-1-Flight-Recorder-v3.md`
- `.GOV/task_packets/WP-1-Flight-Recorder.md`
- `.GOV/task_packets/WP-1-Frontend-AI-Action.md`
- `.GOV/task_packets/WP-1-Frontend-Build-Debug.md`
- `.GOV/task_packets/WP-1-Gate-Check-Tool-v2.md`
- `.GOV/task_packets/WP-1-Gate-Check-Tool.md`
- `.GOV/task_packets/WP-1-Governance-Hooks.md`
- `.GOV/task_packets/WP-1-LLM-Core-v3.md`
- `.GOV/task_packets/WP-1-LLM-Core.md`
- `.GOV/task_packets/WP-1-MCP-End-to-End.md`
- `.GOV/task_packets/WP-1-MCP-Skeleton-Gate.md`
- `.GOV/task_packets/WP-1-Mechanical-Track-Full.md`
- `.GOV/task_packets/WP-1-Metrics-OTel.md`
- `.GOV/task_packets/WP-1-Metrics-Traces.md`
- `.GOV/task_packets/WP-1-MEX-Observability.md`
- `.GOV/task_packets/WP-1-MEX-Safety-Gates.md`
- `.GOV/task_packets/WP-1-MEX-UX-Bridges.md`
- `.GOV/task_packets/WP-1-MEX-v1.2-Runtime-v2.md`
- `.GOV/task_packets/WP-1-MEX-v1.2-Runtime-v3.md`
- `.GOV/task_packets/WP-1-MEX-v1.2-Runtime.md`
- `.GOV/task_packets/WP-1-Migration-Framework.md`
- `.GOV/task_packets/WP-1-Model-Profiles.md`
- `.GOV/task_packets/WP-1-Mutation-Traceability.md`
- `.GOV/task_packets/WP-1-Operator-Consoles-v1.md`
- `.GOV/task_packets/WP-1-Operator-Consoles-v2.md`
- `.GOV/task_packets/WP-1-Operator-Consoles-v3.md`
- `.GOV/task_packets/WP-1-Operator-Consoles.md`
- `.GOV/task_packets/WP-1-OSS-Governance.md`
- `.GOV/task_packets/WP-1-OSS-Register-Enforcement-v1.md`
- `.GOV/task_packets/WP-1-PDF-Pipeline.md`
- `.GOV/task_packets/WP-1-Photo-Studio-Skeleton.md`
- `.GOV/task_packets/WP-1-Photo-Studio.md`
- `.GOV/task_packets/WP-1-RAG-Iterative.md`
- `.GOV/task_packets/WP-1-Retention-GC.md`
- `.GOV/task_packets/WP-1-Security-Gates-v2.md`
- `.GOV/task_packets/WP-1-Security-Gates-v3.md`
- `.GOV/task_packets/WP-1-Security-Gates.md`
- `.GOV/task_packets/WP-1-Semantic-Catalog.md`
- `.GOV/task_packets/WP-1-Spec-Enrichment-LLM-Core-v1.md`
- `.GOV/task_packets/WP-1-Storage-Abstraction-Layer-v2.md`
- `.GOV/task_packets/WP-1-Storage-Abstraction-Layer-v3.md`
- `.GOV/task_packets/WP-1-Storage-Abstraction-Layer.md`
- `.GOV/task_packets/WP-1-Storage-Foundation-20251228.md`
- `.GOV/task_packets/WP-1-Storage-Foundation-v3.md`
- `.GOV/task_packets/WP-1-Storage-Foundation.md`
- `.GOV/task_packets/WP-1-Supply-Chain-MEX.md`
- `.GOV/task_packets/WP-1-Terminal-Integration-Baseline.md`
- `.GOV/task_packets/WP-1-Terminal-LAW-v2.md`
- `.GOV/task_packets/WP-1-Terminal-LAW-v3.md`
- `.GOV/task_packets/WP-1-Terminal-LAW.md`
- `.GOV/task_packets/WP-1-Tokenization-Service-20251228.md`
- `.GOV/task_packets/WP-1-Tokenization-Service-v3.md`
- `.GOV/task_packets/WP-1-Tokenization-Service.md`
- `.GOV/task_packets/WP-1-Validator-Error-Codes-v1.md`
- `.GOV/task_packets/WP-1-Workflow-Engine-v2.md`
- `.GOV/task_packets/WP-1-Workflow-Engine-v3.md`
- `.GOV/task_packets/WP-1-Workflow-Engine-v4.md`
- `.GOV/task_packets/WP-1-Workflow-Engine.md`
- `.GOV/task_packets/WP-1-Workspace-Bundle.md`
- `.GOV/task_packets/stubs/README.md`
- `.GOV/task_packets/stubs/WP-1-ACE-Auditability-v2.md`
- `.GOV/task_packets/stubs/WP-1-ACE-Runtime-v2.md`
- `.GOV/task_packets/stubs/WP-1-AI-Job-Model-v4.md`
- `.GOV/task_packets/stubs/WP-1-AI-UX-Actions-v2.md`
- `.GOV/task_packets/stubs/WP-1-AI-UX-Rewrite-v2.md`
- `.GOV/task_packets/stubs/WP-1-AI-UX-Summarize-Display-v2.md`
- `.GOV/task_packets/stubs/WP-1-Atelier-Lens-v2.md`
- `.GOV/task_packets/stubs/WP-1-Calendar-Lens-v2.md`
- `.GOV/task_packets/stubs/WP-1-Canvas-Typography-v2.md`
- `.GOV/task_packets/stubs/WP-1-Capability-SSoT-v2.md`
- `.GOV/task_packets/stubs/WP-1-Cross-Tool-Interaction-Conformance-v1.md`
- `.GOV/task_packets/stubs/WP-1-Dev-Experience-ADRs.md`
- `.GOV/task_packets/stubs/WP-1-Distillation-v2.md`
- `.GOV/task_packets/stubs/WP-1-Editor-Hardening-v2.md`
- `.GOV/task_packets/stubs/WP-1-Flight-Recorder-UI-v3.md`
- `.GOV/task_packets/stubs/WP-1-Global-Silent-Edit-Guard.md`
- `.GOV/task_packets/stubs/WP-1-Governance-Kernel-Conformance-v1.md`
- `.GOV/task_packets/stubs/WP-1-Governance-Hooks-v2.md`
- `.GOV/task_packets/stubs/WP-1-LocalFirst-Agentic-MCP-Posture-v1.md`
- `.GOV/task_packets/stubs/WP-1-MCP-End-to-End-v2.md`
- `.GOV/task_packets/stubs/WP-1-MCP-Skeleton-Gate-v2.md`
- `.GOV/task_packets/stubs/WP-1-Metrics-OTel-v2.md`
- `.GOV/task_packets/stubs/WP-1-Metrics-Traces-v2.md`
- `.GOV/task_packets/stubs/WP-1-MEX-Observability-v2.md`
- `.GOV/task_packets/stubs/WP-1-MEX-Safety-Gates-v2.md`
- `.GOV/task_packets/stubs/WP-1-MEX-UX-Bridges-v2.md`
- `.GOV/task_packets/stubs/WP-1-Migration-Framework-v2.md`
- `.GOV/task_packets/stubs/WP-1-Model-Profiles-v2.md`
- `.GOV/task_packets/stubs/WP-1-Mutation-Traceability-v2.md`
- `.GOV/task_packets/stubs/WP-1-OSS-Governance-v2.md`
- `.GOV/task_packets/stubs/WP-1-PDF-Pipeline-v2.md`
- `.GOV/task_packets/stubs/WP-1-Photo-Studio-v2.md`
- `.GOV/task_packets/stubs/WP-1-RAG-Iterative-v2.md`
- `.GOV/task_packets/stubs/WP-1-Response-Behavior-ANS-001.md`
- `.GOV/task_packets/stubs/WP-1-Semantic-Catalog-v2.md`
- `.GOV/task_packets/stubs/WP-1-Spec-Router-Session-Log.md`
- `.GOV/task_packets/stubs/WP-1-Supply-Chain-MEX-v2.md`
- `.GOV/task_packets/stubs/WP-1-Workspace-Bundle-v2.md`
- `.GOV/templates/AI_WORKFLOW_TEMPLATE.md`
- `.GOV/templates/REFINEMENT_TEMPLATE.md`
- `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`
- `.GOV/templates/TASK_PACKET_TEMPLATE.md`

### 9.8 `.GOV/operator/docs_local/`
- `.GOV/operator/docs_local/DOC_INDEX.txt`
- `.GOV/operator/docs_local/Diary RID extraction.txt`
- `.GOV/operator/docs_local/legacy/The_Prompt_Diaries_v03.056.000_2025-11-28_01-42_CET.txt`


