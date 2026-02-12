# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in
  `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Spec-Enrichment-Product-Governance-Consistency-v1

## STUB_METADATA
- WP_ID: WP-1-Spec-Enrichment-Product-Governance-Consistency-v1
- BASE_WP_ID: WP-1-Spec-Enrichment-Product-Governance-Consistency
- CREATED_AT: 2026-02-11T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER (non-authoritative): Phase 1 runtime integration + governance kernel adoption
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - 2.3.15 Locus Work Tracking System (integration points + task board sync)
  - 2.6.6.8 Micro-Task Executor Profile (iteration lifecycle + escalation)
  - 4.3.7 Work Profile System (role-based assignment + governance knobs)
  - 4.3.9 Multi-Model Orchestration & Lifecycle Telemetry (role identity + lifecycle markers)
  - 7.5.4 Governance Kernel / Governance Pack / Template Volume (project-agnostic export)
  - 7.5.4.10 Product Governance Snapshot (HARD)

## PROBLEM_STATEMENT (DRAFT)
- The Master Spec contains internal drift where product runtime state is described in one place as
  product-owned (e.g. `.handshake/gov/`), but other sections still cite repo-local paths such as:
  - `docs/TASK_BOARD.md`
  - `docs/task_packets/{WP_ID}.md`
- This drift reintroduces confusion between:
  - Repo governance workspace used to BUILD Handshake (this repo), and
  - Product governance/state that Handshake MUST own for arbitrary projects/workspaces.
- Result: inconsistent guidance for implementers and models, and a high probability of boundary
  regressions (product accidentally depending on repo governance files).

## INTENT (DRAFT)
- What: Produce a spec-only correction pass that makes product governance/state rules internally
  consistent, project-agnostic, and disk-agnostic across all affected sections and examples.
- Why: A spec that contradicts itself guarantees recurring regressions, role confusion, and
  non-deterministic "governance drift" during implementation.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Normalize terminology:
    - "Repo governance workspace" vs "Product runtime governance state" must be defined once and
      used consistently.
  - Normalize canonical runtime paths and reference types:
    - Task Board: define the canonical runtime Task Board reference (path or artifact ref) and
      prohibit repo-local `docs/` references as runtime dependencies.
    - Task Packets: define canonical runtime references (avoid repo `.GOV/` and `docs/` paths as
      runtime dependencies; prefer artifact handles or runtime-owned paths).
  - Update Locus integration points:
    - `locus_sync_task_board` MUST operate on runtime-owned state (not repo `docs/`).
    - Work Packet -> Task Packet linking must not require repo paths.
  - Update Micro-Task Executor / escalation references where they cite repo artifacts or assume
    Handshake-specific layouts.
  - Add explicit conformance assertions (spec-level) that can be mechanically checked:
    - "Spec MUST NOT require runtime reads/writes of repo `.GOV/**` or `docs/**`."
  - Add a "compatibility appendix" if needed:
    - If `docs/**` bundles exist, they MUST be explicitly labeled compat-only, one-way export, and
      non-authoritative for runtime correctness.
- OUT_OF_SCOPE:
  - Any product code changes.
  - Any repo governance workflow refactors (protocol/scripts).
  - Renaming existing WP IDs or moving historical task packets.

## ACCEPTANCE_CRITERIA (DRAFT)
- The Master Spec no longer cites `docs/TASK_BOARD.md` or `docs/task_packets/` as runtime sources
  of truth in Locus integration points.
- The Master Spec has exactly one canonical definition of runtime governance state root semantics
  (disk-agnostic, under workspace root, no parent-dir traversal), and all examples match it.
- The Master Spec explicitly distinguishes:
  - Repo `.GOV/**` as repo governance workspace (used to author/build/validate this repo), and
  - Product-owned runtime governance state (used by Handshake in arbitrary workspaces).
- Conformance section includes at least one mechanical check spec for preventing boundary
  regressions (even if implemented later).

## VALIDATOR_RUBRIC_HOOKS (DRAFT)
- Determinism: no absolute-path examples; one canonical path vocabulary; examples are self-consistent.
- Boundary: spec does not require runtime reads/writes of repo `.GOV/**` or `docs/**`.
- Portability: rules and examples apply to arbitrary projects/workspaces (not Handshake-specific layouts).
- Operability: conformance checks are specified in a way that can be mechanized later (grepable invariants).

## VALIDATION_PLAN (DRAFT)
- Spec-only:
  - Prove internal consistency by scanning the spec for legacy runtime path examples:
    - `rg -n "docs/TASK_BOARD\\.md|docs/task_packets/" Handshake_Master_Spec_*.md`
  - Prove runtime path semantics are consistent by scanning for the canonical runtime root token
    and ensuring it is used in the Locus/Micro-Task sections:
    - `rg -n "\\.handshake/gov|HANDSHAKE_GOVERNANCE_ROOT" Handshake_Master_Spec_*.md`

## RED_TEAM / ABUSE_CASES (DRAFT)
- RT-BOUNDARY-001: Spec includes a single leftover `docs/` example that later becomes a runtime
  dependency in code.
- RT-BOUNDARY-002: Spec defines runtime governance state in two places with conflicting rules
  (one allows `.GOV`, one forbids it).
- RT-DETERMINISM-001: Spec examples imply absolute paths or host-specific drive letters.
- RT-TRACE-001: Locus links to task packets via repo-local paths, breaking project agnosticism and
  causing missing artifacts in non-Handshake repos.

## DEPENDENCIES / BLOCKERS (DRAFT)
- None. This is a spec-only correction/enrichment WP.

## RISKS / UNKNOWNs (DRAFT)
- Risk: touching multiple spec sections without a single authoritative "path vocabulary" leads to
  partial fixes. Mitigation: introduce a single "Governance State Vocabulary" subsection and
  reference it everywhere.
- Risk: decisions about task packet refs (path vs artifact handle) require agreement. Mitigation:
  specify a stable ref type in the spec first; code can follow later.

## PROPOSED_SPEC_ENRICHMENT_SKELETON (DRAFT)
- Add a single normative "Governance State Vocabulary" subsection in the Master Spec that defines:
  - Repo Governance Workspace: files under repo `.GOV/**` (and any `docs/**` bundles) used only to
    author/build/validate Handshake itself; NOT runtime dependencies.
  - Product Runtime Governance State Root: the workspace-owned, project-agnostic root directory
    where Handshake persists governance state for arbitrary projects (examples must be drive-agnostic).
- Add explicit invariants (mechanically checkable):
  - Runtime MUST NOT read/write repo `.GOV/**` or `docs/**` as part of correctness.
  - Runtime MUST NOT require absolute paths or host-specific drive letters in canonical examples.
- Task Board + Work Packets references:
  - Define a canonical runtime Task Board reference type (path or artifact handle), and require
    Locus/Micro-Task integrations to use ONLY that runtime reference.
  - Define canonical runtime "Task Packet reference" semantics that do not require repo-local paths.
- Compatibility note (only if needed):
  - If a `docs/**` export bundle exists, it is explicitly one-way export, non-authoritative, and
    MUST NOT be used as runtime source of truth.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm anchor sections exist in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Spec-Enrichment-Product-Governance-Consistency-v1` (in `.GOV/task_packets/`).
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
