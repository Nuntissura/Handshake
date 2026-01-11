# Master Spec MVP + Roadmap Audit (v02.103)

## Scope
- Spec read: `Handshake_Master_Spec_v02.103.md` (full file, top-to-bottom).
- Governance artifacts consulted:
  - `docs/SPEC_CURRENT.md`
  - `docs/TASK_BOARD.md`
  - `docs/ROADMAP_VS_MASTER_SPEC_AUDIT_v02.102.md`
  - `docs/MASTER_SPEC_INTENT_AUDIT_v02.103.md`
- Goal:
  1) Identify what the current Master Spec requires for the Product MVP.
  2) Check whether the Roadmap (Spec §7.6) represents the Master Spec without loss of intent / hidden technical debt.
  3) Identify phase-structure risks (missing work or confusing phase usage).

## Repo state (hard gate)
- Worktree: `D:\Projects\LLM projects\wt-orchestrator`
- Branch: `user_orchestrator`

## Deterministic checks run (summary)
1) Locate Roadmap bounds in the spec:
   - Roadmap section detected as `## 7.6 Development Roadmap` .. `## 8.1 Risk Assessment`
2) Extract and compare Work Packet IDs (WP-*) across:
   - Spec total vs spec roadmap vs spec non-roadmap
   - Spec WP set vs Task Board WP set
3) Count "MVP + normative keyword" statements outside the Roadmap (to see whether MVP is defined in Main Body vs Roadmap).
4) Scan for internal "PHASE n" diagrams that may conflict with the global Roadmap phase numbering.
5) Deep inspection (non-Roadmap): section-by-section digest and normative scan:
   - Generated digest: `docs/MASTER_SPEC_SECTION_DIGEST_v02.103.md` (Purpose + line-numbered MUST/SHOULD requirements per `## X.Y` section; Roadmap pointer mentions by phase).
   - Generated Roadmap section coverage matrix: `docs/ROADMAP_SECTION_COVERAGE_MATRIX_v02.103.md` (All Main Body sections -> whether the Roadmap cites the literal section number).
   - Generated Phase 1 evidence map: `docs/PHASE_1_EVIDENCE_MAP_v02.103.md` (Phase 1 MUST deliver -> Main Body anchors -> Task Board WPs).

## 1) What is needed for an MVP (per Master Spec v02.103)

### 1.1 Product MVP definition location
- The spec's concrete definition of "Product MVP (Phase 1)" is in Spec §7.6.3 (Roadmap).
- Outside the Roadmap section, the only explicit "MVP + MUST/SHOULD/MUST NOT" clauses are in the ASR subsystem (Spec §6.2) and describe the ASR MVP boundaries (which are scheduled for Phase 3 in §7.6.5).

Implication:
- If you strictly ignore §7.6 (Roadmap), the spec does not currently provide a complete, MUST-level "Product MVP" checklist for the core product. The MVP checklist for the core product effectively lives in the Roadmap text.

### 1.2 Product MVP (Phase 1) - high-level MUST areas (Spec §7.6.3)
The Roadmap's Phase 1 "MUST deliver" bundle for the Product MVP includes (grouped, not verbatim):
- Local model runtime integration (default Ollama config; at least one bundled/preloaded model; APIs for chat + context).
- Global AI Job Model (schema, job kinds/states, profiles) implemented and used.
- Workflow & Automation Engine used as the only production path for AI work (durable state in SQLite; queue/store).
- Capability + consent enforcement wired end-to-end (no-bypass; write-gating + validation; persistence of consent fields).
- Flight Recorder always-on, with Operator Console UI surfaces (Job History, Timeline, Problems/Evidence); DuckDB-backed log store.
- Deterministic edit process + "No Silent Edits" UX primitives for rewrite flows (diff + accept/reject).
- Mechanical Tool Bus / Mechanical Extension v1.2 wiring and global engine gates (conformance harness; denials visible).
- Diagnostics/export deliverables (Debug Bundle + Workspace Bundle; PDF pipeline via Typst + qpdf).
- Terminal LAW minimal slice (terminal runs are governed: capabilities + workflow + provenance + Flight Recorder).
- Canvas typography + offline font packs and deterministic rendering/export behavior.
- Spec Router + governance session log behavior (git-backed safety commits only when appropriate).
- ACE runtime validator artifacts (ContextPlan/ContextSnapshot) and validator pack gating.
- Photo Studio skeleton as a first-class governed surface (artifact-first; no direct file mutation).

For the full normative checklist, refer directly to Spec §7.6.3.

## 2) Technology stack (spec vs repo reality)

### 2.1 Spec-stated stack (high signal excerpts)
- The spec repeatedly treats the "Coordinator" as a Rust/Tauri host process (e.g., Rust coordinator + IPC + local stores), and defines normative Rust types for core schemas (e.g., JobKind as a Rust enum).
- Spec §8.2 includes a "Technology Stack Summary" table that lists "Backend: Python (FastAPI)" and "AI Runtime: Ollama, ComfyUI".

### 2.2 Repo reality (this repo)
- Backend implementation is Rust: `src/backend/handshake_core` is a Rust crate (`Cargo.toml`, `src/`, `migrations/`).
- Frontend is TypeScript/React with Tauri: `app/package.json` includes `@tauri-apps/*`, React, Tiptap, Excalidraw, Yjs.

### 2.3 Key alignment risk
- Spec §8.2's "Backend: Python (FastAPI)" does not match the current repo's backend language choice (Rust).
- Elsewhere, the spec does describe a Rust coordinator and allows for external services, so the simplest explanation is: §8.2 is stale or describes an optional/alternative component.

Operator decision needed (governance-level):
- Clarify whether the authoritative "backend/coordinator" for this repo is Rust-only, or Rust coordinator + optional Python service.
- If §8.2 is meant to be authoritative, it should be updated via the Spec Enrichment workflow (new spec version + `docs/SPEC_CURRENT.md` update + signature audit), not as an ad-hoc edit.

## 3) Does the Roadmap represent the Master Spec 100%?

### 3.1 What can be checked deterministically today (and passed)
Work Packet ID coverage:
- WP IDs found in the v02.103 spec: 6 total:
  - `WP-1-Storage-Abstraction-Layer`
  - `WP-1-AppState-Refactoring`
  - `WP-1-Migration-Framework`
  - `WP-1-Dual-Backend-Tests`
  - `WP-1-Capability-SSoT`
  - `WP-1-Global-Silent-Edit-Guard`
- All 6 WP IDs appear in the Roadmap (§7.6) and are represented in `docs/TASK_BOARD.md`.

Existing intent audits:
- `docs/ROADMAP_VS_MASTER_SPEC_AUDIT_v02.102.md` and `docs/MASTER_SPEC_INTENT_AUDIT_v02.103.md` document recent Roadmap pointer gap fixes (e.g., missing ANS-001 scheduling) and add a "no technical debt" phase-closure bullet to the Roadmap preamble.

### 3.2 What fails the strictest reading of "100% representation"
Even with the Roadmap preamble asserting it is "pointer only", the Roadmap itself contains many MUST-level requirements (Phase 0/1/2/3/4 "MUST deliver" lists).

This creates an inherent risk:
- If a requirement exists only in §7.6 and not in the governing Main Body sections, then "ignore Roadmap" would lose intent.
- Conversely, if "Done" is defined solely by Main Body, then Roadmap MUST-lists are either redundant (should be pointers) or they are silently authoritative (contradicting the "pointer only" rule).

Net: As written, the Roadmap is not a pure pointer layer; it contains normative content.

### 3.3 Phase-numbering ambiguity inside the spec (risk of planning errors)
The spec includes subsystem-internal diagrams that use "PHASE 1/2/3" naming that conflicts with the global Roadmap phases:
- Sync topology diagram (Spec §3.3.2.2) uses:
  - "PHASE 1 (MVP): File Sync + CRDT"
  - "PHASE 2 (Multi-user): Central Sync Server"
- Plugin architecture appendix (Spec §8.6.3.4) uses:
  - "PHASE 1 (MVP): Internal Extension Points", "PHASE 2: User Scripts", "PHASE 3: Full Plugin System"

These are not necessarily wrong, but they are ambiguous because §7.6 defines Phase 1..4 for the whole product. This is a concrete place where roadmap-vs-spec planning can drift.

## 4) Are the phases utilized well? (high-level assessment)

Strengths:
- §7.6 defines an explicit Phase 0..4 sequence that covers the major technical subsystems (governance + diagnostics early; ingestion/ASR later; collaboration last).
- The Roadmap preamble now explicitly encodes the "no technical debt / Main Body compliance" closure rule (added in v02.103).

Risks / likely missing clarity:
- Phase numbering conflicts (Section 3 and Appendix 8.6) can cause mistaken assumptions about when sync/plugins are expected to ship.
- If the project truly requires Roadmap MUST-lists as normative, then those requirements should be moved/duplicated into governing Main Body sections (or the governance rule about which sections are authoritative should be revised), otherwise "pointer only" is not enforceable without losing intent.

---

## Addendum (v02.105) — Deeper inspection + Roadmap Coverage Matrix phase allocation

**Spec baseline updated:** `Handshake_Master_Spec_v02.105.md` (see `docs/SPEC_CURRENT.md`).

### What changed in v02.105 (roadmap governance)
- §7.6.1 Coverage Matrix rules tightened:
  - Phase 0 is closed: the matrix must not allocate any row to `P0` (remediate in `P1+`).
  - No placeholders: `UNSCHEDULED` is no longer permitted; every row must have at least one phase allocation.
- Coverage Matrix now phase-allocates every non-Roadmap section (section-level determinism) across `P1–P4`.
- Roadmap phase Goal blocks now explicitly reference the §7.6.1 Coverage Matrix as the scheduling authority.

### Roadmap ↔ Main Body coverage (global / all phases)
At section-level, **the Roadmap is now deterministically complete** because §7.6.1 lists every `## X.Y` outside §7.6 (plus top-level `# 9.`) exactly once and assigns it to one or more phases.

Deterministic matrix validation (v02.105; run 2026-01-11):
- Expected section rows: 68 (67 `## X.Y` excluding §7.6 + top-level `# 9.`)
- Matrix rows: 68
- Duplicates: 0
- Missing: 0
- Extra: 0
- Title mismatches: 0
- Invalid phase tokens: 0

Remaining caveat (unchanged from v02.103):
- The Roadmap section (§7.6) still contains many MUST-level statements (Phase "MUST deliver" lists). Governance currently treats Main Body (§1-§6, §9-§11) as the authority for "Done" (CX-598), and the Roadmap as scheduling/pointer. This means:
  - Section-level completeness does not automatically imply requirement-level completeness.
  - Phase closure still requires evidence mapping against the governing Main Body sections, not only checking Roadmap bullets.

## 5) Suggested next step (if you want strict 100% alignment)
If you want a deterministic "no intent loss" proof:
- Create an "Evidence Map" document that maps each Phase 1 Roadmap MUST-deliver item to:
  - the governing Main Body SPEC_ANCHOR(s), and
  - the concrete WP ID(s) on `docs/TASK_BOARD.md`.
- For any Roadmap MUST that has no Main Body anchor, decide whether to:
  - promote it into Main Body via Spec Enrichment (new version), or
  - downgrade it to a pointer-only line.

Status:
- Evidence map generated: `docs/PHASE_1_EVIDENCE_MAP_v02.103.md` (Phase 1 Roadmap MUST-deliver -> Main Body anchors -> Task Board WPs).
- Additive remediation stubs created for Ready-for-Dev FAIL items: see `docs/TASK_BOARD.md` + `docs/task_packets/stubs/`.

## 6) Deep inspection addendum (sections + normative density; Roadmap excluded)

### 6.1 Digest artifact
- `docs/MASTER_SPEC_SECTION_DIGEST_v02.103.md` is a deterministic extraction of:
  - Purpose per `## X.Y` section (`**Why**` / `**What**` blocks when present; otherwise the first intro lines).
  - All lines containing `MUST`, `SHOULD`, `REQUIRED`, `MUST NOT`, `SHOULD NOT` (with `Handshake_Master_Spec_v02.103.md:<line>` pointers).
  - Whether each section number is mentioned in the Roadmap, and which phases mention it by number.

### 6.2 Normative density (non-Roadmap)
Summary (excluding `Handshake_Master_Spec_v02.103.md:20065` .. `Handshake_Master_Spec_v02.103.md:21465`):
- `##` sections (excluding Roadmap): 67
- Total normative lines detected (excluding Roadmap): 830
- Sections with >=1 normative line: 34 (33 sections have none by keyword scan)
- Top sections by normative-line count:
  - `2.6 Workflow & Automation Engine` (146)
  - `6.2 Speech Recognition: ASR Subsystem` (111)
  - `2.5 AI Interaction Patterns` (81)
  - `2.3 Content Integrity (COR-700)` (68)
  - `6.3 Mechanical Extension Engines` (47)
  - `10.5 Operator Consoles` (47)
  - `10.1 Terminal Experience` (45)
  - `11.4 Diagnostics Schema` (36)
  - `11.7 OSS Component Choices & Versions` (33)
  - `2.2 Data & Content Model` (24)

### 6.3 Roadmap pointer coverage (by literal section-number mentions)
This is a strict pointer check: does the Roadmap mention the authoritative section number (e.g., `11.4`), not just the concept words.
- Sections whose `X.Y` number is mentioned somewhere in Roadmap: 34/67
- Among sections with >=1 normative line: 21/34 are mentioned by number; 13/34 are not

Highest-normative sections not referenced by number in the Roadmap (potential pointer clarity gaps):
- `11.4 Diagnostics Schema (Problems/Events)` (36 lines)
- `11.5 Flight Recorder Event Shapes & Retention` (15 lines)
- `10.10 Photo Studio` (18 lines)
- `2.4 Extraction Pipeline (The Product)` (6 lines)
- `4.2 LLM Inference Runtimes` (8 lines)
- `4.4 Image Generation (Stable Diffusion)` (8 lines)

Interpretation:
- These are not automatically "missing features" (the Roadmap may still describe them by name), but they are gaps in explicit pointer-citation, which increases drift risk when trying to prove "no intent loss" deterministically.
