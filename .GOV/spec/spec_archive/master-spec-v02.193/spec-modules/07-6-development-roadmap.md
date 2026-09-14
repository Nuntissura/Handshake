---
schema: handshake.indexed_spec.module@1
spec_version: "v02.193"
bundle_id: "master-spec-v02.193"
module_id: "07-6"
section_id: "7.6"
title: "7.6 Development Roadmap"
source_baseline_version: "v02.182"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "3309d38c73ae0afc91572ad4fe41e83bf8738f1fe77847e28b589ee3d8a4f0b2"
body_sha256: "dff08aee62ad0e77ef5c5f992fa4325682d1770c62b6245ed8bb9fea3b5275a7"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
<a id="76-development-roadmap"></a>
## 7.6 Development Roadmap

**Roadmap tag provenance notes (non-normative)**
- **v02.186 note:** KERNEL-004 4-cluster max-fold enrichment merged into the Main Body (Â§3.5, Â§3.6, Â§4.2.4 rewrite, Â§4.6, Â§4.7, Â§4.8, Â§5.6, Â§5.7, Â§6.4, Â§6.5, Â§6.6, Â§6.7, Â§6.8, Â§10.12, Â§10.13, Â§10.14, Â§10.15) and reflected here per CX-128. Roadmap updates are **additive only** and tagged `[ADD v02.186]`; no new phase-template fields per CX-128/v02.128. Phase 0 remains closed.
- **v02.139 note:** Promptâ†’Spec hardening quartet added to the Main Body (SpecPromptPack + SpecPromptCompiler, CapabilitySnapshot, SpecLint). Roadmap updates are **additive only** and tagged `[ADD v02.139]`.

- **v02.130 note:** Loom integration spec merged into master spec (LoomBlock/LoomEdge + Loom views + schemas/events). Roadmap updates are **additive only** and tagged `[ADD v02.130]`.
- **v02.131 note:** Handshake Stage spec merged into master spec (Â§10.13). Roadmap updates are **additive only** and tagged `[ADD v02.131]`.
- **v02.136 note:** Unified Tool Surface Contract (HTC v1.0, Â§6.0.2) + Tool Gate (single enforcement point) were normalized into the Main Body. Roadmap updates are **additive only** and tagged `[ADD v02.136]`.
- **v02.136 note:** `Handshake_Design_Studio_Overhaul_v0.1.md` is treated as a **UI/IA recontextualization** of existing modules/worksurfaces (not a replacement of Handshake). Roadmap entries schedule the shell/IA shift in Phase 2+ to avoid Phase 1 rework.
- **v02.129 note:** Roadmap normalization pass: converted legacy `**ADD v02.xxx â€” â€¦**` atomic blocks into inline phase-field patches; version tags preserved; no scope change.
- **v02.127 note:** Sidecar tech integration spec merged into master spec as Â§10.11 (Dev Command Center). Roadmap updates are **additive only** and tagged `[ADD v02.127]` (Phase 0 remains closed). New implementation focus: workspaces=git worktrees + WP linkage, Approval Inbox (capability gating UX), Execution Session Manager, Objective Anchor Store + Handoff records, and governed VCS/search/run queues via `engine.version`/`engine.context`/`engine.sandbox` (MEX v1.2; no-bypass).
- **v02.120 note:** Runtime Integration Addendum v0.5 merged into master spec. Roadmap updates are **additive only** and tagged `[ADD v02.120]` (Phase 0 remains closed). New implementation focus: ModelSwap/resource management, Work Profiles, AutomationLevel autonomous governance + GovernanceDecision audit, Role Mailbox (â€œInboxâ€) alignment + runtime telemetry, cloud escalation consent artifacts/events, and promptâ†’macroâ†’micro pipeline integration.
- **v02.123 note:** Atelier/Lens Addendum v0.2.3 merged into Â§6.3.3.5.7 as Â§6.3.3.5.7.11â€“Â§6.3.3.5.7.25. Roadmap updates are **additive only** and tagged `[ADD v02.123]` (Phase 0 remains closed). New implementation focus: Tier1/Tier2 extraction depth separation, Tier2 auto-when-idle scheduling, SYM-001 template growth + profile drift/branching, selection-scoped Atelier collaboration, and hard-drop SFW projection.
- **v02.36 note:** Additive roadmap entries are tagged `[ADD v02.36]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.40 note:** Additive roadmap entries are tagged `[ADD v02.40]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.42 note:** Additive roadmap entries are tagged `[ADD v02.42]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.49 note:** Additive roadmap entries are tagged `[ADD v02.49]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.52 note:** Additive roadmap entries are tagged `[ADD v02.52]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.63 note:** Additive roadmap entries are tagged `[ADD v02.63]` (reconciliation of orphans; no rewrites of prior bullets).
- **v02.101 note:** Additive roadmap entries are tagged `[ADD v02.101]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.102 note:** Additive roadmap entries are tagged `[ADD v02.102]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.103 note:** Additive roadmap entries are tagged `[ADD v02.103]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.104 note:** Additive roadmap entries are tagged `[ADD v02.104]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.105 note:** Additive roadmap entries are tagged `[ADD v02.105]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.115 note:** Additive roadmap entries are tagged `[ADD v02.115]` (no rewrites of existing bullets; Phase 0 remains closed). **Flight Recorder remediation:** FR-EVT-DATA-001..015 events for AI-Ready Data Architecture (Â§2.3.14) are NEW scope that requires updating the existing Flight Recorder implementationâ€”this is remediation work, not new infrastructure. **Skill Bank remediation:** LoRA training data flow from retrieval quality signals (Â§2.3.14.9, Â§9) requires extending existing Skill Bank schemas.
- **v02.116 note:** Roadmap entries tagged `[ADD v02.116]` (Locus Work Tracking System, Â§2.3.15) are **open to revision**. All Task Board entries tagged `v02.116` MUST be reviewed and revised/updated to match current Locus WP ID + status semantics and the updated Phase bullets.
**Why**  
A clear roadmap with phases and dependencies ensures focused effort and prevents scope creep. This section provides the practical build order for Project Handshake.

**What**  
Defines four development phases (Foundation, Core Editing, AI Integration, Visual Tools + Polish), specifies MVP scope, and shows the dependency graph for build order. Each phase now explicitly carries a **Mechanical Track** so engines from Section 6 are woven in, not deferred.

**Jargon**  
- **MVP (Minimum Viable Product)**: The smallest set of features that delivers value and validates the concept.
- **IPC (Inter-Process Communication)**: How the frontend and backend processes communicate.
- **Phase 0**: Foundation work (monorepo, scaffolding, CI pipeline, basic IPC).

---

**Why**  
Handshake is intentionally ambitious: local models, workflows, governance, ingestion, ASR, collaboration. A roadmap is required to sequence this complexity into phases that are buildable, testable, and debuggable. The goal is to ensure that governance (Diary, AI Job Model, capabilities), observability (Flight Recorder, metrics, traces), migrations, and debug tools are present from the start, not bolted on after features ship.

**What**  
This section defines a phased implementation plan for Handshake, from a pre-MVP foundation to a multi-user, extensible workspace. Each phase specifies what MUST be shipped, what is explicitly out of scope, and:

- A **vertical slice** (end-to-end user flow) that proves the phase is real, not just infra.  
- Key **risks** the phase should reduce.  
- **Acceptance criteria** that define â€œDoneâ€, including debug and diagnostic surfaces.
Cross-ref: roadmap phases should account for Terminal/Monaco delivery (Section 10) and shared capability/observability contracts (Section 11) as part of milestones and acceptance.

All phases are aligned with the architecture and mechanisms defined in Sections 2â€“6 (Architecture, Data Model, LLM Infrastructure, Observability, Mechanical Integrations).

**Jargon**  
- **Pre-MVP (Phase 0)** â€“ Foundation work that produces a running but non-compelling app; used to validate architecture, tooling, and debug surfaces.  
- **Product MVP (Phase 1)** â€“ First version that a single user can use for serious work, with governance and full diagnostic surfaces.  
- **Phase** â€“ A coherent bundle of changes that is shippable, testable, and has a clear vertical slice.  
- **Core loop** â€“ The smallest end-to-end user flow that exercises architecture and observability: â€œedit doc â†’ ask AI â†’ see changes + history + logs.â€  
- **Shadow Workspace** â€“ Background index and graph over workspace content used for search and RAG.  
- **Flight Recorder** â€“ Append-only event log for AI jobs, workflows, and user-visible actions, used for debugging and audit.  
- **AI Job** â€“ A single AI operation with ID, profile, capabilities, inputs, outputs, lifecycle, and provenance, as defined by the global AI Job Model.  
- **Debug surface** â€“ Any UI, log view, trace viewer, or health check that makes it possible for a human to understand and diagnose system behaviour without reading the entire codebase.  
- **Vertical slice** - A thin, end-to-end scenario that exercises UI, backend, data, and observability in one flow.  
- **Mechanical job** - A deterministic operation executed by an external engine (CAD, search, media, etc.) through the Gate/Body pattern, producing artifacts with sidecar provenance.

---

### 7.6.1 Roadmap Coverage Matrix (Section-Level Determinism) (HARD)

#### 7.6.1.1 Scope and Principles

This roadmap applies to the **entire Handshake product**, not just subsystems.

It MUST:

- Align with the architectural layers in Section 2 (Architecture), Section 3 (Data Model), Section 4 (LLM Infrastructure), Section 5 (Observability & Benchmarks), and Section 6 (Mechanical Integrations).  
- Ensure that **AI Job Model**, **Workflow & Automation Engine**, **Flight Recorder**, and the **capability system** are exercised early and consistently.  
- Deliver a **single-user, local-first, offline-capable** product before adding multi-user sync, plugins, or cloud dependencies.  
- Treat **Docling** and **ASR** as extensions on top of the same AI Job and workflow mechanisms, not separate systems.  
- [ADD v02.139] Treat Promptâ†’Spec as a governed, reproducible pipeline: Spec Router MUST use SpecPromptPack+SpecPromptCompiler, emit CapabilitySnapshot, and enforce SpecLint preflight before rubric/red-team and execution.  
- Require that every user-facing feature ships with a **diagnostic path**:
  - Logs or events in Flight Recorder.  
  - At least one debug surface (UI, CLI, or trace) that shows how it behaves.  
- Use each phase to **burn down risk**, not only to add features.  
- Provide **clear acceptance criteria** per phase: conditions and tests that must pass before moving on.  
- [ADD v02.103] **Phase closure rule (HARD, no technical debt):** The roadmap is a pointer only; a phase is complete only when all governing Master Spec Main Body requirements (Sections 1â€“6 and 9â€“11) for that phase are implemented and validated. A Vertical slice is necessary but not sufficient.
- Use a fixed **phase template** (fields, in order):
  - **Goal**
  - **MUST deliver**
  - **Key risks addressed in Phase n**
  - **Mechanical Track (Phase n)** (if present)
  - **Atelier Track (Phase n)** (if present)
  - **Distillation Track (Phase n)** (if present)
  - **Vertical slice**
  - **Acceptance criteria**
  - **Explicitly OUT of scope**
  - **Status** (only when a phase is closed)
- [ADD v02.122] **Do not add a permanent â€œAddendumâ€ section.** Place content into the topic where it belongs and update roadmap matrices and cross-references accordingly.
- [ADD v02.105] **No privileged fields:** For phase closure, every line in a phase section is equal importance; "MUST deliver" does not override or exclude other fields (Key risks, Tracks, Vertical slice, Acceptance criteria, Explicitly OUT of scope).
- [ADD v02.128] **No new roadmap phase fields:** Do not create new permanent phase-template blocks/sections. Weave new work into the fixed phase template fields (Goal/MUST deliver/Key risks/Tracks/Vertical slice/Acceptance/Out of scope).
- [ADD v02.128] **Roadmap reflection rule (HARD):** Any normative addition merged into the Master Spec (including subsection-level imports that do not create new `## X.Y` headings) MUST be reflected in the Roadmap the same change by adding `[ADD vXX]` bullets in the relevant Phase(s) that point to the new subsection(s).

#### 7.6.1.2 Coverage Matrix (HARD)

- [ADD v02.104] Established the section-level Coverage Matrix (audit-first) to prevent Roadmap drift.
- [ADD v02.105] Phase-allocated all matrix rows (P1-P4) and removed Phase 0 allocations to reflect that Phase 0 is closed.
- [ADD v02.138] Front End Memory System (FEMS) merged as **subsection-level** patches under Â§2.6.6.6.6, Â§2.6.6.7.6.2, Â§4.3.9.12.7, Â§5.4.8, Â§10.11.5.14, and Â§11.5.13; Coverage Matrix rows remain unchanged because it tracks `X.Y` (and top-level `# X.`) headings.

This Roadmap MUST include and maintain a **section-level Coverage Matrix** that prevents Roadmap drift.

**Definitions**
- **Section-level**: The unit is a Master Spec section number (`X.Y`) and any top-level `# X.` section that has no `## X.Y` headings (currently `# 9.`).
- **Main Body Authority (CX-598)**: Sections in Â§1â€“Â§6 and Â§9â€“Â§11 are the authoritative definition of â€œDoneâ€. The Roadmap is scheduling/pointer only; it does not redefine authority.

**Rules (HARD)**
1. The Coverage Matrix MUST list every `## X.Y` section outside Â§7.6 AND the top-level `# 9.` section.
2. Every listed section MUST appear **exactly once** in the matrix.
3. The matrix MUST include:
   - `Section` (exact number)
   - `Title` (exact heading text)
   - `Main Body Authority (CX-598)` (YES/NO)
   - `Roadmap Coverage (Phase(s))` (allowed values: `P1`, `P2`, `P3`, `P4` or a comma-separated list).
4. Any edit that adds/removes/renumbers sections MUST update the matrix in the same change.
5. [ADD v02.128] Subsection-level normative imports (no new `## X.Y` row) MUST still update the Roadmap in the same change:
   - Add `[ADD vXX]` bullets to the relevant Phase(s) that point to the new subsection numbers (do not rely on the parent row alone).
   - Add a note bullet above this table documenting the patch and confirming which existing matrix row covers it.
6. Missing rows, duplicate rows, or incorrect section numbers are a **BLOCKING governance failure**.
7. Phase 0 is closed: the matrix MUST NOT allocate any row to `P0`. Any newly discovered Phase 0 gaps MUST be scheduled in `P1` or later.
8. The matrix MUST NOT use placeholders like `UNSCHEDULED`; every row MUST have at least one phase allocation.

**Coverage Matrix (v02.105 baseline; phase-allocated; Phase 0 closed)**
- [ADD v02.123] Atelier/Lens Addendum v0.2.3 merged as a **subsection-level** patch under Â§6.3.3.5.7; Coverage Matrix rows are unchanged (no new `## X.Y` sections added) and phase allocations remain valid.
- [ADD v02.114] Â§2.6.6.8 Micro-Task Executor Profile added as subsection of Â§2.6; covered by existing Â§2.6 row (P1, P2, P3, P4).
- [ADD v02.115] Â§2.3.14 AI-Ready Data Architecture added; cross-cutting integration with all tool sections (Â§10.x).
- [ADD v02.120] Runtime Integration Addendum v0.5 merged into existing sections (not new top-level `##` rows): model swaps (Â§4.3), Work Profiles (Â§4.3), AutomationLevel governance (Â§2.6.8), cloud escalation consent (Â§11.1), event catalog additions (Â§11.5), and debug bundle exports (Â§10.5).
- [ADD v02.128] Â§2.6.8.13 Spec Creation System v2.2.1 imported as a **subsection-level** patch under Â§2.6.8; Coverage Matrix rows are unchanged (covered by existing Â§2.6 row), but Phase bullets are updated to schedule implementation.
- [ADD v02.127] Â§10.11 Dev Command Center (Sidecar Integration) added as new Product Surface; Coverage Matrix updated with new row (Phase 0 closed).
- [ADD v02.136] Â§6.0.2 Unified Tool Surface Contract (HTC v1.0) imported as a **subsection-level** patch under Â§6.0; Coverage Matrix rows are unchanged (covered by existing Â§6.0 row), but Phase bullets are updated to schedule implementation and prevent â€œdual tool schemaâ€ drift.
- [ADD v02.152] Spec Router / Locus / MCP / MEX backend evidence-projection deepening lands as subsection-level patches under Â§2.3, Â§2.6, Â§6.0, and Â§11.5/Â§11.8; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 deepen backend orchestration, projection, and replay contracts.
- [ADD v02.153] Capability / diagnostics backend evidence deepening lands as subsection-level patches under Â§2.3, Â§2.6, Â§11.1, and Â§11.5; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 deepen capability enforcement, recorder correlation, diagnostics materialization, and consent artifact portability.
- [ADD v02.154] Governance Pack / Workspace Bundle backend export reciprocity lands as subsection-level patches under Â§6.0, Â§7.5.4, and Â§10.5.7; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 backfill governance-export feature ownership, recorder visibility, and storage-portability edges.
- [ADD v02.155] Calendar backend force-multiplier deepening lands as subsection-level patches under Â§2.6.6.8, Â§10.4, and Appendix 12; Coverage Matrix rows remain unchanged while Phase 1 bullets deepen Calendar storage-portability, consent, AI-job mutation, and ACE / Spec Router routing posture.
- [ADD v02.156] Knowledge/retrieval pillar deepening lands as subsection-level patches under Â§2.3.13, Â§2.5.8, Â§2.5.12, and Â§2.6.7; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 deepen retrieval-substrate ownership, deterministic routing registry posture, portable ContextPack artifacts, and Loom storage portability.
- [ADD v02.159] Correlation/projection backend deepening lands as subsection-level patches under Â§10.5, Â§10.11, and Appendix 12; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 clarify Dev Command Center vs Operator Consoles ownership, deepen Role Mailbox projection posture, and add control/projection edges into Flight Recorder and Debug Bundle.
- [ADD v02.164] Dev Command Center resilience and repository-decision deepening lands as subsection-level patches under Â§4.3.9, Â§6.3.10, Â§10.11, and Appendix 12; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 deepen session recovery, provider readiness, anti-pattern surfacing, and version-control backend policy.
- [ADD v02.165] Dev Command Center operating-surface deepening lands as subsection-level patches under Â§4.3.9, Â§6.0.2, Â§6.3.10, Â§10.11, and Appendix 12; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 deepen run history, tool infrastructure health, workspace runtime readiness, and promotion-gate snapshots.
- [ADD v02.166] Structured collaboration-substrate deepening lands as subsection-level patches under Â§2.3.15, Â§2.6.6.8, Â§2.6.8.8, Â§2.6.8.10, Â§7.2, Â§10.11, and Appendix 12; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 deepen structured work records, append-only notes, collaboration-inbox routing, and Dev Command Center field viewers.
- [ADD v02.179] Workflow-correlation bundle-scope deepening lands as subsection-level patches under Ã‚Â§2.3, Ã‚Â§2.6, and Ã‚Â§10.5 plus Appendix 12.5; Coverage Matrix rows remain unchanged while Phase 1 bullets and FEAT-DEBUG-BUNDLE guidance deepen workflow-run and node-execution export posture.
- [ADD v02.181] Software-delivery governance overlay boundary deepening lands as subsection-level patches under 7.5.4.8 and 7.5.4.9 plus Appendix 12; Governance Pack import/export remains the repo-facing transfer surface while repository `/.GOV/**` artifacts stay imported overlay source material or evidence rather than live runtime authority.
- [ADD v02.181] Software-delivery runtime-truth deepening lands as subsection-level patches under 2.3.15, 2.6.8.8, 2.6.8.10, and 7.2 plus Appendix 12; Coverage Matrix and UI guidance deepen `FEAT-LOCUS-WORK-TRACKING`, `FEAT-ROLE-MAILBOX`, and `FEAT-WORKFLOW-ENGINE` around product-owned runtime records, workflow-backed governed actions, and stable-id-linked software-delivery state.
- [ADD v02.181] Validator-gate and closeout deepening lands as subsection-level patches under 2.3.15, 7.2, 7.5.4.9, and 10.11 plus Appendix 12; Phase 1 bullets and Appendix 12 clarify validator-gate convergence, gate-summary projection, evidence-linked gate execution, and derived closeout posture instead of packet-surgery closeout truth.
- [ADD v02.181] Projection-surface deepening lands as subsection-level patches under 2.6.8.8, 2.6.8.10, 7.2, and 10.11 plus Appendix 12; Coverage Matrix and UI guidance deepen `FEAT-DEV-COMMAND-CENTER`, `FEAT-ROLE-MAILBOX`, and `FEAT-LOCUS-WORK-TRACKING` so Dev Command Center, Task Board, and Role Mailbox remain projection and control surfaces over the same runtime truth without mailbox chronology or Markdown mirrors becoming authority.
- [ADD v02.181] Overlay coordination-record deepening lands as subsection-level patches under 2.3.15, 7.2, and 10.11 plus Appendix 12; Phase 1 bullets and primitive coverage add overlay claim/lease posture and queued-instruction posture so takeover, steering, follow-up, and actor-eligibility decisions can be modeled without relying on ad hoc comments or transcript order.
- [ADD v02.181] Overlay lifecycle and control-plane deepening lands as subsection-level patches under 2.3.15, 7.2, and 10.11 plus Appendix 12; Phase 1 bullets and Appendix 12 clarify lifecycle checkpoints, checkpoint-backed recovery posture, and workflow-backed start/steer/cancel/close/recover control-plane semantics.
- **Note:** Phase 0 is now closed. No new work items may be allocated to Phase 0; any remaining Phase 0 stubs MUST be either completed as part of Phase 1 or explicitly moved to a later phase.

| Section | Title | Main Body Authority (CX-598) | Roadmap Coverage (Phase(s)) |
|---|---|---|---|
| 1.1 | Executive Summary | YES | P1, P2, P3, P4 |
| 1.2 | The Diary Origin Story | YES | P1, P2, P3, P4 |
| 1.3 | The Four-Layer Architecture | YES | P1, P2, P3, P4 |
| 1.4 | LLM Reliability Hierarchy | YES | P1, P2, P3, P4 |
| 1.5 | What Gets Ported from the Diary | YES | P1, P2, P3, P4 |
| 1.6 | Design Philosophy: Self-Enforcing Governance | YES | P1, P2, P3, P4 |
| 1.7 | Success Criteria | YES | P1, P2, P3, P4 |
| 1.8 | Introduction | YES | P1, P2, P3, P4 |
| 2.1 | High-Level Architecture | YES | P1, P2, P3, P4 |
| 2.2 | Data & Content Model | YES | P1, P2, P3, P4 |
| 2.3 | Content Integrity (Diary Part 5: COR-700) | YES | P1, P2, P3, P4 |
| 2.3.14 | AI-Ready Data Architecture [ADD v02.115] | YES | P1, P2, P3, P4 |
| 2.3.15 | Locus Work Tracking System [ADD v02.116] | YES | P1, P2, P3, P4 |
| 2.4 | Extraction Pipeline (The Product) | YES | P2, P3, P4 |
| 2.5 | AI Interaction Patterns | YES | P1, P2, P3, P4 |
| 2.6 | Workflow & Automation Engine | YES | P1, P2, P3, P4 |
| 2.7 | Response Behavior Contract (Diary ANS-001) | YES | P1, P2, P3, P4 |
| 2.8 | Governance Runtime (Diary Parts 1-2) | YES | P1, P2, P3, P4 |
| 2.9 | Deterministic Edit Process (COR-701) | YES | P1, P2, P3, P4 |
| 2.10 | Session Logging (LOG-001) | YES | P1, P2, P3, P4 |
| 3.1 | Local-First Data Fundamentals | YES | P1, P2, P3, P4 |
| 3.2 | CRDT Libraries Comparison | YES | P4 |
| 3.3 | Database & Sync Patterns | YES | P4 |
| 3.4 | Conflict Resolution UX | YES | P4 |
| 4.1 | LLM Infrastructure | YES | P1, P2, P3, P4 |
| 4.2 | LLM Inference Runtimes | YES | P1, P2, P3, P4 |
| 4.3 | Model Selection & Roles | YES | P1, P2, P3, P4 |
| 4.3.9 | Multi-Model Orchestration & Lifecycle Telemetry [ADD v02.122] | YES | P1, P2, P3, P4 |
| 4.4 | Image Generation (Stable Diffusion) | YES | P2, P3, P4 |
| 4.5 | Layer-wise Inference & Dynamic Compute (Exploratory) [ADD v02.122] | YES | P1, P3, P4 |
| 5.1 | Plugin Architecture | YES | P4 |
| 5.2 | Sandboxing & Security | YES | P1, P2, P3, P4 |
| 5.3 | AI Observability | YES | P1, P2, P3, P4 |
| 5.4 | Evaluation & Quality | YES | P1, P2, P3, P4 |
| 5.5 | Benchmark Harness | YES | P2, P3, P4 |
| 6.0 | Mechanical Tool Bus & Integration Principles | YES | P1, P2, P3, P4 |
| 6.1 | Document Ingestion: Docling Subsystem | YES | P2, P3, P4 |
| 6.2 | Speech Recognition: ASR Subsystem | YES | P3, P4 |
| 6.3 | Mechanical Extension Engines | YES | P1, P2, P3, P4 |
| 7.1 | User Interface Components | NO | P1, P2, P3, P4 |
| 7.2 | Multi-Agent Orchestration | NO | P1, P2, P3, P4 |
| 7.3 | Collaboration and Sync | NO | P4 |
| 7.4 | Reference Application Analysis | NO | P1, P2 |
| 7.5 | Development Workflow | NO | P1, P2, P3, P4 |
| 8.1 | Risk Assessment | NO | P1 |
| 8.2 | Technology Stack Summary | NO | P1 |
| 8.3 | Gap Analysis & Open Questions | NO | P1 |
| 8.4 | Consolidated Glossary | NO | P1 |
| 8.5 | Sources Referenced | NO | P1 |
| 8.6 | Appendices | NO | P1 |
| 8.7 | Version History & Subsection Versioning | NO | P1 |
| 9 | Continuous Local Skill Distillation (Skill Bank & Pipeline) | YES | P1, P2, P3, P4 |
| 10.1 | Terminal Experience | YES | P1, P2, P3, P4 |
| 10.2 | Monaco Editor Experience | YES | P1, P2, P3, P4 |
| 10.3 | Mail Client | YES | P3, P4 |
| 10.4 | Calendar | YES | P1, P2, P3, P4 |
| 10.5 | Operator Consoles: Debug & Diagnostics | YES | P1, P2, P3, P4 |
| 10.6 | Canvas: Typography & Font Packs | YES | P1 |
| 10.7 | Charts & Dashboards | YES | P1, P2, P3, P4 |
| 10.8 | Presentations (Decks) | YES | P1, P2, P3, P4 |
| 10.9 | Future Surfaces | YES | P4 |
| 10.10 | Photo Studio | YES | P1, P2, P3, P4 |
| 10.11 | Dev Command Center (Sidecar Integration) | YES | P1, P2, P3, P4 |
| 10.12 | Loom (Heaper-style Library Surface) | YES | P1, P2, P4 |
| 10.13 | Handshake Stage (Built-in Browser + Stage Apps) [ADD v02.131] | YES | P1, P2, P3, P4 |
| 11.1 | Capabilities & Consent Model | YES | P1, P2, P3, P4 |
| 11.2 | Sandbox Policy vs Hard Isolation | YES | P1, P2, P3, P4 |
| 11.3 | Auth/Session/MCP Primitives | YES | P1, P2, P3, P4 |
| 11.4 | Diagnostics Schema (Problems/Events) | YES | P1, P2, P3, P4 |
| 11.5 | Flight Recorder Event Shapes & Retention | YES | P1, P2, P3, P4 |
| 11.6 | Plugin/Matcher Precedence Rules | YES | P4 |
| 11.7 | OSS Component Choices & Versions | YES | P1, P2, P3, P4 |
| 11.8 | Mechanical Extension Specification v1.2 (Verbatim) | YES | P1, P2, P3, P4 |
| 11.9 | Future Shared Primitives | YES | P4 |
| 11.10 | Implementation Notes: Phase 1 Final Gaps | YES | P1 |

- Treat completed phases as **closed**: if requirements are discovered late, they MUST be scheduled into the **current or later** phase (never retroactively added to a finished phase).  
- Preserve a **migration path**: schema and config changes must respect existing data where possible.
- Treat **Mechanical Engines** as first-class: every phase ships at least one mechanical job through the Workflow Engine with capability gates, Flight Recorder logging, and reproducible commands/artifacts.

Out of scope for this section:

- Detailed team planning (sprints, owners, ticket breakdown).  
- Budget and resourcing assumptions.  
- Full QA test plans (see Sections 5.4â€“5.5 instead).

---

### 7.6.2 Phase 0 â€” Foundations (Pre-MVP)

**Status**  
Closed (completed). No new scope may be added to Phase 0; newly discovered requirements MUST be scheduled in Phase 1 or later.

**Goal**  
Stand up a stable â€œHello, workspaceâ€ application that matches the high-level architecture and establishes baseline logging, health checks, and a reproducible dev environment. No serious AI or governance yet, but debug tooling MUST already exist.

**MUST deliver**

1. **Desktop shell and process model**  
   - Tauri-based desktop application with a React front-end.  
   - Backend orchestrator process started and managed by the desktop shell.  
   - Canonical IPC/API channel between frontend and backend (HTTP/WebSocket/IPC), documented and testable.

2. **Workspace and data layer (single user)**  
   - SQLite workspace database with minimal schema for:
     - Workspaces / projects.  
     - Documents and blocks (honouring the Raw/Derived/Display split, even if only Raw/Display are used initially).  
     - Canvases, nodes, and edges.  
   - Basic, tested CRUD operations for documents and canvases.  
   - Initial schema migration mechanism (even simple, versioned migrations) so the DB can evolve safely.

3. **Editors and navigation**  
   - Rich text editor integrated in the main content area with:
     - Headings, paragraphs, lists, code blocks, quotes, inline marks.  
   - Canvas view integrated with:
     - Sticky notes / text boxes, simple shapes, arrows, pan/zoom.  
   - Workspace sidebar listing documents and canvases; open/save loop must be reliable.

4. **Project health, logging, and basic debug tools**  
   - Monorepo and tooling per Section 7.5 (linting, formatting, tests, CI) wired to this stack.  
   - Structured logging in frontend and backend (log level, context, correlation IDs where applicable).  
   - A **health check** endpoint or command that verifies at least:
     - App shell â†’ backend connectivity.  
     - Database connectivity.  
   - A simple developer-facing log view (tail of logs or structured log output) suitable for non-expert developers.  
   - One-command dev startup (script or target that starts frontend, backend, and DB with sample data).

**Vertical slice**  
- Start the app.  
- Create a workspace.  
- Create a document and a canvas, make simple edits, close the app, reopen, and verify content is intact.  
- Run the health check and inspect logs to confirm basic operations are recorded.

**Key risks addressed in Phase 0**

- Stack (Tauri + frontend + backend + DB) is unstable or too hard to run.  
- No consistent logging/health model, making later debugging painful.  
- Schema and migrations are ad-hoc from day one.

**Acceptance criteria**

- App can be started and used locally by running a single documented command.  
- Health check succeeds in a clean environment.  
- Logs clearly show at least workspace creation, document creation, and document save events.  
- A sample workspace can be created, exported (or re-created), and used as a fixture for later phases.

**Explicitly OUT of scope**

- AI Job Model, workflows, Flight Recorder, Shadow Workspace.  
- Multi-user sync or CRDT.  
- Docling, ASR, connectors, plugin system.
- Mechanical engines beyond a stub: only scaffolding and a single proof mechanical job for wiring.

**Mechanical Track (Phase 0)**
- Deliver a **mechanical runner** abstraction (deterministic process exec with resource limits) and capability flags for file/process/cpu/memory.
- Ship one **stub mechanical job** (e.g., `Context` engine using `rg`) through the Workflow Engine with Flight Recorder logging and sidecar provenance (command, exit code, stdout/stderr, artifact hash).
- Acceptance: mechanical job visible in Job History; blocked when capability is missing; artifact stored as DerivedContent if produced.

---

### 7.6.3 Phase 1 â€” Core Product MVP (Single-User, Local AI)

**Goal**  
Deliver the **first real Handshake**: a single-user, local-first workspace where documents and canvases are editable, AI assistance is available, and every AI action is traceable through the AI Job Model, Workflow Engine, Flight Recorder, and capability system. Debug tools for AI behaviour and workflows are mandatory.
- [ADD v02.136] Unify tool invocation: local tool calling and MCP MUST use the same Tool Registry + Tool Gate + Flight Recorder event model (no bypass).
Ship with the default Handshake-native ModelRuntime path, hardened document/canvas editors (Tiptap/BlockNote and Excalidraw), always-on Atelier Lens, and a prompt-to-spec router that creates Task Board and Work Packet session logs. Git workflows must trigger safety commit behavior while non-git workflows must not.

- [ADD v02.139] Promptâ†’Spec hardening quartet (Phase 1): Spec Router MUST compile prompts via SpecPromptPack+SpecPromptCompiler (PromptEnvelope + ContextSnapshot), MUST inject CapabilitySnapshot (explicit allowlist), and MUST enforce SpecLint preflight gate (G-SPECLINT) before rubric/red-team and MT decomposition.

- [ADD v02.123] Phase 1 Atelier/Lens focus: ship selection-scoped collaboration, Tier1-first extraction with deterministic role-turn isolation, and ViewMode hard-drop projection; Tier2 auto-when-idle is scheduled in Phase 2.

- [ADD v02.105] Phase coverage is governed by Â§7.6.1 Coverage Matrix; Phase 0 is closed and MUST NOT be used for scheduling newly discovered requirements (remediate in Phase 1 or later).
- [ADD v02.107] Governance kernel adoption (workflow safety): Phase 1 work MUST be executed using the Governance Kernel artifacts and gates (AÂ§7.5.4) so small-context local models and cloud models can hand off deterministically (Task Board + refinements + signed task packets + manifests; CI parity).
- [ADD v02.107] Local-first agentic stance: Phase 1 core loops MUST run fully offline with local models + local tools as the default; MCP and cloud escalation are optional, capability-gated, and must preserve artifact-first outputs and Flight Recorder evidence (AÂ§6.0.1, AÂ§7.2.5, AÂ§11.3).
- [ADD v02.52] ACE-RAG-001 groundwork: make retrieval planning/tracing/budgeting a first-class runtime contract in MVP (QueryPlan + RetrievalTrace + strict budgets + cache keys). ContextPacks/drift/caching effectiveness ship in Phase 2.
- [ADD v02.52] Add: Minimum viable export to prevent lock-in and enable reproducible debugging (bundles).

- [ADD v02.68] Adopt Mechanical Extension v1.2 as the callable Tool Bus contract for all mechanical jobs (Engine PlannedOperation/EngineResult envelopes, artifact-first I/O, no-bypass).
- [ADD v02.68] Require v1.2 global engine gates + conformance harness before expanding mechanical engines beyond MVP wiring (denials/reasons visible in Problems + Flight Recorder).

- [ADD v02.79] Establish **Photo Studio (skeleton)** as a first-class workspace surface governed by AI Job Model + Workflow Engine + Flight Recorder (no-bypass; job-request driven).
- **[ADD v02.130] Loom (Heaper-style library surface) MVP**  
  Deliver a local-first Loom library surface: LoomBlocks (note/file/context), four views (All/Unlinked/Sorted/Pins), and inline @mention/#tag organization with backlinks.
- **[ADD v02.131] Handshake Stage MVP (governed browser + capture surface)**  
  Deliver Handshake Stage as a first-class, governed browser surface with isolated sessions/tabs, embedded Stage Apps, and a Stage Bridge API that can enqueue evidence-backed capture/import jobs (web/PDF/3D) through the Workflow Engine + Mechanical Tool Bus (no bypass).




- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**  
  Make multi-model execution a first-class, governed workflow capability: multiple independent model instances can execute different WPs/MTs in parallel, with strict file-scope locks, deterministic recovery, and compact lifecycle telemetry.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**  
  Deliver a single, canonical control surface that binds **Locus work (WP/MT)** â†” **git workspaces (worktrees)** â†” **execution sessions** â†” **approvals/logs/diffs**, enabling safe parallel development with deterministic recovery and auditability. It is the default place to observe/approve governed tool calls (Tool Gate) and export deterministic debug bundles.

- [ADD v02.142] Runtime visibility contract (Phase 1): every runtime-visible capability touched in Phase 1 MUST land with Appendix 12 capability slices + runtime visibility rows so Command Center, Flight Recorder, Locus, and storage posture are explicit before scope expansion.
- [ADD v02.143] Primitive index coverage contract (Phase 1): every Appendix 12.3 feature touched in Phase 1 MUST land with a normalized Appendix 12.4 coverage row (arrays only, coverage_status, coverage_refs, gap_stub_ids) before coding or further roadmap expansion.
- [ADD v02.144] Second-pass feature-family coverage (Phase 1): explicitly named feature families and runtime shells discovered during refinement MUST be promoted into Appendix 12.3 / 12.4 / 12.5 / 12.6 with stub-backed gap tracking before further matrix expansion.
- [ADD v02.145] Third-pass runtime/data/operator coverage (Phase 1): execution-path features and reusable operator/export/filter/session contracts discovered during refinement MUST be promoted into Appendix 12 with runtime visibility rows and interaction edges before wider matrix expansion continues.
- [ADD v02.146] Deepening pass (Phase 1): seeded rows with real UI/operator, event-taxonomy, export-query, or retrieval-artifact contracts MUST be upgraded in Appendix 12 before widening the feature frontier again.
- [ADD v02.147] Orphan-ownership pass (Phase 1): high-signal orphan primitives for capability/consent, jobs, debug/export, storage portability, and operator projection MUST be attached to owning feature rows or resolved to stubs before new feature-family expansion resumes.
- [ADD v02.166] Structured collaboration substrate (Phase 1): canonical Work Packet, Micro-Task, Task Board, and Role Mailbox state MUST be readable as structured records and rendered through Dev Command Center field viewers; human-readable Markdown remains a mirror or note sidecar instead of the only machine-readable source.
- [ADD v02.167] Canonical structured artifact family (Phase 1): Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports MUST use versioned JavaScript Object Notation or JavaScript Object Notation Lines canonical files with compact summaries, project-agnostic base envelopes, and explicit Markdown mirror-sync rules; Dev Command Center board and queue layouts remain derived projections over those records.
- [ADD v02.168] Base structured schema and project-profile contracts (Phase 1): Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports MUST share one base structured-collaboration envelope with stable record identity, mirror-state semantics, authoritative references, compact summaries, and explicit project-profile extension boundaries so later Handshake kernels can reuse the same artifact family safely.
- [ADD v02.169] Canonical-to-mirror reconciliation and drift governance (Phase 1): Markdown mirrors, note sidecars, and projected board views MUST reconcile against canonical structured records through explicit mirror contracts, authority modes, reconciliation actions, and drift-safe Dev Command Center controls so readable surfaces never become silent competing authorities.
- [ADD v02.170] Dev Command Center typed viewer, board layout, and queue projection (Phase 1): board, queue, list, roadmap, inbox-triage, and execution-queue surfaces MUST be driven by explicit view presets, lane definitions, and governed action bindings; local-small-model readiness queues MUST stay compact-summary-first; and drag or quick actions MUST preview authoritative mutations before execution.
- [ADD v02.171] Project-agnostic workflow-state and governed-action contracts (Phase 1): Work Packets, Micro-Tasks, Task Board projections, Role Mailbox-linked queues, and Dev Command Center operating views MUST share one base workflow-state family, queue-reason vocabulary, and governed-action descriptor contract so routing, triage, review, approval, validation, and completion semantics stay stable across future Handshake project kernels.
- [ADD v02.172] Workflow transition matrix, queue automation, and executor eligibility contracts (Phase 1): the shared workflow-state vocabulary MUST now be paired with explicit transition rules, automatic queue-action rules, and executor eligibility policies so local small models, cloud models, reviewers, workflow automation, and operators can mutate work only through portable, inspectable, approval-aware transitions.
- [ADD v02.173] Role Mailbox message contract, thread lifecycle, and authority boundary (Phase 1): Role Mailbox MUST separate thread lifecycle from message delivery state, encode allowed-response envelopes plus due and snooze posture, carry typed Micro-Task collaboration message families, and request linked work changes only through governed actions or explicit transcription instead of transcript-order heuristics.
- [ADD v02.174] Role Mailbox and Micro-Task loop control (Phase 1): Role Mailbox MUST coordinate verifier-driven Micro-Task retry, feedback, verification-needed, escalation, and completion-report traffic through bounded loop checkpoints, structured verifier outcomes, remaining retry budget, and explicit escalation targets; and Work Packet, Task Board, Locus, and Dev Command Center views MUST project that state without replaying full threads or treating mailbox chronology as authority.
- [ADD v02.175] Role Mailbox triage, queue aging, and remediation controls (Phase 1): Role Mailbox MUST expose durable triage queue state, reminder schedules, snooze and expiry posture, dead-letter disposition, and explicit operator remediation controls; Dev Command Center and Task Board MUST project mailbox pressure, queue age, and remediation posture without treating unread state or thread order as authority; and linked Work Packet, Micro-Task, and Locus views MUST explain mailbox-derived waiting or recovery posture through stable identifiers plus explicit queue reasons.
- [ADD v02.176] Role Mailbox executor routing, claim-lease, and response authority (Phase 1): Role Mailbox MUST expose executor-kind allowlists, claim or lease modes, claimant identity, lease expiry, takeover policy, and response-authority scope for actionable threads; Dev Command Center and Task Board MUST project claimant, lease age, actor-ineligible posture, and takeover legality without turning mailbox claims into work-state authority; and linked Locus, Micro-Task, and Work Packet views MUST explain who may act next through stable identifiers plus governed-action previews.
- [ADD v02.177] Role Mailbox handoff bundle, note transcription, and announce-back provenance (Phase 1): Role Mailbox MUST carry structured handoff bundles and announce-back provenance for delegate, handoff, completion, scope-change, and escalation traffic; Work Packet, Locus, Micro-Task, Task Board, and Dev Command Center views MUST surface remaining work, unresolved blockers, recommended next actor, and transcription status without replaying entire threads; and advisory announce-back messages MUST NOT imply authoritative completion until linked transcription or governed action is confirmed.
- [ADD v02.178] Governed RAG retrieval modes and no-RAG policy (Phase 1): AI-Ready Data, ACE Runtime, Project Brain, Prompt-to-Spec Router, Loom, Work Packets, and Micro-Task Executor MUST share explicit retrieval-mode selection (`none`, `direct_load`, `exact_lookup`, `graph_traversal`, `hybrid_rag`); exact authoritative ids MUST bypass broad hybrid retrieval by default; `QueryPlan` and `RetrievalTrace` MUST record non-hybrid reasons; and local-small-model loops MUST keep hybrid retrieval opt-in, bounded, and compaction-first.
- [ADD v02.179] Workflow-correlation bundle scopes (Phase 1): Debug Bundle export and operator workflow views MUST support explicit `workflow_run` and `workflow_node_execution` scopes, portable `workflow_node_executions.jsonl` inventory, and validator-visible lineage ids so replay and diagnostics stay bounded without time-window reconstruction.
- [ADD v02.148] Ownership-reduction deepening (Phase 1): Stage/media session-auth contracts, multi-session runtime substrate, and debug/export/retention contracts discovered in code MUST be attached to FEAT-STAGE, FEAT-MEDIA-DOWNLOADER, FEAT-MODEL-SESSION-ORCHESTRATION, FEAT-DEBUG-BUNDLE, and FEAT-STORAGE-PORTABILITY before broader orphan hunting resumes.
- [ADD v02.149] Refinement reciprocity hardening (Phase 1): Main Body<->Appendix reciprocity, roadmap/coverage-matrix coupling, mandatory matrix research, mandatory GUI implementation advice, `[ADD v<version>]` packet visibility, primitive exposure/creation reporting, and stub creation for new spec/roadmap entries not absorbed in scope MUST be enforced before broader matrix expansion.
- [ADD v02.150] Backend-heavy matrix expansion (Phase 1): workflow projection correlation, consent audit projection, calendar correlation export, and Stage/media artifact portability MUST be modeled as first-class backend combo tracks with Appendix 12.6 edges and stub-backed follow-on work before frontend-led matrix breadth resumes.
- [ADD v02.151] Backend export/evidence/portability expansion (Phase 1): Role Mailbox, AI-Ready Data, and Workflow Engine MUST surface direct Flight Recorder/storage portability links; unresolved mailbox/debug-bundle, AI-ready/debug-bundle, and calendar-mailbox bridges remain stub-backed until explicit backend contracts land.
- [ADD v02.152] Backend orchestration/projection/replay expansion (Phase 1): Spec Router, Locus, MCP Gate, and MEX Runtime MUST surface direct Flight Recorder/debug-bundle/storage-portability links; unresolved spec-router portability, locus-debug-bundle, and MCP/MEX evidence-export bridges remain stub-backed until explicit backend contracts land.
- [ADD v02.153] Backend capability/diagnostic evidence expansion (Phase 1): workflow capability checks, spec-router capability snapshots, MCP recorder events, and diagnostics bundle materialization MUST surface direct Flight Recorder/debug-bundle/capability links; unresolved cloud-consent evidence portability remains stub-backed until explicit manifest/retention contracts land.
- [ADD v02.154] Backend governance/export reciprocity expansion (Phase 1): Governance Pack and Workspace Bundle export surfaces MUST be appendix-visible backend features with direct Workflow Engine / capability / Flight Recorder / storage-portability posture; unresolved bundle instantiation and workspace-transfer delivery remain stub-backed until full implementation lands.
- [ADD v02.155] Calendar backend force-multiplier expansion (Phase 1): Calendar MUST remain an appendix-visible backend source-state, capability, AI-job mutation, and scope-hint routing surface with direct Storage Portability / Capabilities & Consent / AI Job Model / Spec Router posture; richer mailbox, Locus, and debug-bundle bridges remain stub-backed until concrete backend contracts land.
- [ADD v02.156] Knowledge/retrieval pillar expansion (Phase 1): Project Brain, Semantic Catalog, Context Packs, and Loom MUST remain appendix-visible backend retrieval contracts with direct AI-Ready Data / Spec Router / Storage Portability posture; weaker graph-to-notebook and export-driven bridges remain stub-backed until explicit backend contracts land.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**  
  Introduce stable, governed **dynamic compute** hooks (no algorithm requirement) so future phases can experiment with layer-wise inference without breaking auditability or determinism.

**MUST deliver**

1. **Model runtime integration (LLM core)**  
   - Integrate one Handshake-native local LLM runtime as specified in Section 4.  
   - Configure at least one general-purpose model.  
   - Ship a default MVP runtime configuration using **Handshake ModelRuntime** with at least one preloaded or runtime-discovered general-purpose model (e.g., **Llama 3 13B** or **Mistral-7B**) enabled out of the box.  
   - Backend API for:
     - Chat-style requests with system + user prompts.  
     - Passing document context (selected text, document snapshot or summary).  

   - [ADD v02.120] Implement sequential model swaps (ModelSwapRequest) with state persistence + ACE recompile; emit FR-EVT-MODEL-*.
   - [ADD v02.120] Implement Work Profiles (role-based model assignment + automation knobs); record `work_profile_id` in job metadata; emit FR-EVT-PROFILE-*.
   - [ADD v02.120] Integrate cloud escalation consent artifacts (ProjectionPlan + ConsentReceipt) + FR-EVT-CLOUD-*; enforce human-gated escalation.
   - [ADD v02.120] Align â€œInboxâ€ UI label to Role Mailbox subsystem; add runtime mailbox telemetry + debug bundle export coverage.




2. **AI Job Model (minimum viable implementation)**  
   - Implement the **global AI Job Model** (Section 2.6.6) in the backend:
     - `job_id`, `job_kind`, `protocol_id`, `status`, timestamps, error, inputs, outputs, metrics.  
     - Profile fields (`profile_id`, `capability_profile_id`, `access_mode`, `safety_mode`).  
   - Implement a **Docs AI Job profile subset** compatible with the Docs & Sheets profile:
     - `doc_id`, selection/range selector, layer scope, provenance fields linking edits back to source content and user.  

3. **Workflow & Automation Engine (minimum viable)**  
   - Implement the Workflow Engine core (Section 2.6) with:
     - Single-node workflows representing one AI job.  
     - Durable state in SQLite (workflow run + job records).  
     - SQLite serves as the authoritative AI job queue/store for MVP workflows.  
     - Status transitions: `queued â†’ running â†’ completed / failed`.  
   - All AI work MUST go through the Workflow Engine; no direct â€œcall the modelâ€ shortcuts are allowed in production paths.

4. **Capability and consent enforcement (minimal slice)**  
   - Define a minimal capability set for documents:
     - `doc.read`, `doc.write`, `doc.summarize` (at minimum).  
   - Every AI job MUST declare required capabilities.  
   - The Workflow Engine MUST enforce that:
     - Jobs without `doc.write` cannot mutate documents.  
     - Write operations are applied only after passing validation (even if the MVP uses a deterministic, auto-accept validator).  
   - Consent-related fields MUST be persisted, even if the MVP uses a simple â€œglobal consentâ€ toggle.

5. **Flight Recorder (always-on, with UI)**  
   - Implement a Flight Recorder subsystem (Section 2.1.5 and Bootloader clauses) with:
     - Append-only log of AI job lifecycle events.  
     - Append-only log of model calls (model name, tokens, latency, outcome).  
     - Minimal tags to correlate events (job ID, workflow ID, document ID, user ID where applicable).  
     - Back the Flight Recorder log store with **DuckDB** to support filtered queries at MVP scale.  
   - Provide a **Job History** panel in the UI:
     - List jobs with status, timestamps, model used, and linked document.  
     - Ability to inspect job input and output payloads.  
     - Provide a basic Flight Recorder filter for `job_id` and `status` to quickly locate related runs.
     - Provide an **Operator Consoles v1** surface (see Â§10.5):
       - **Timeline** view (Flight Recorder events with filters + deep links).
       - **Jobs** view (Job History + per-job inspector).
       - **Problems** view (normalized diagnostics, grouped/deduped, clickable evidence).
       - **Evidence drawer** that shows: job summary, linked trace slice, linked diagnostics, and referenced entities/files.
     - Provide **Debug Bundle export** (redacted-by-default) for a selected `job_id` or time range:
       - Includes: `job.json`, `trace.jsonl`, `diagnostics.json`, `env.json` (no secrets), and `repro.md`.
       - Export action MUST emit a Flight Recorder event and be capability-gated.
  

6. **Baseline metrics and traces (debugging AI behaviour)**  
   - Export basic metrics for:
     - Request counts and error counts.  
     - Latency distribution per action (no target values required here).  
     - Token usage per job/model.  
   - Attach simple trace identifiers to AI jobs and workflow runs so that:
     - A single user action can be followed across model calls and internal steps.  
   - Provide at least one way to view or export these diagnostics (e.g. a debug UI panel or log-based trace view).
   - Implement the **normalized Diagnostic pipeline** (DIAG-SCHEMA-001/002):
     - Deterministic fingerprinting + dedup/grouping rules so repeated failures collapse into a single Problem with a count.
     - Correlate diagnostics to `job_id`, `workflow_id`, `wsid`, and `activity_span_id` where available.
   - Ship a **validator pack** wired into CI:
     - Diagnostic schema validation (required fields, ranges, stable IDs).
     - Flight Recorder event shape validation (minimum linkability fields).
     - Debug Bundle completeness + SAFE_DEFAULT redaction check.
  
   - Instrument AI Job and Workflow engines with **OpenTelemetry** (or compatible SDK) to emit latency, error rate, and token-count metrics as part of the MVP diagnostics surface.  

7. **AI UX in the editor (basic actions)**  
   - Command Palette actions:
     - "Ask about this document" (chat with context).  
     - "Summarize document."  
   - Inline actions:
     - "Rewrite selection" for document text.  
   - All actions MUST:
     - Create AI jobs with the correct profile and capabilities.  
     - Execute via the Workflow Engine.  
     - Persist results back into documents through structured patches.  
     - Emit events into Flight Recorder; the corresponding jobs must appear in Job History and in metrics/traces.  
     - Run on hardened editor components: `Tiptap`/`BlockNote` for documents and `Excalidraw` for canvas interactions, wired through the same AI job/capability/logging pathways.  

8. **Governance hooks (Diary alignment)**  
   - Store enough metadata on jobs and workflows to later map them to Diary RIDs and clauses (activation, modes, gates).  
   - Enforce the invariant: **no silent AI edits**. Every AI mutation of user content MUST be traceable to a specific job and workflow run.  
   - Add basic Bootloader/Diary compliance checks to CI to prevent regressions in logging or observability.

   - [ADD v02.120] Implement AutomationLevel + GovernanceDecision/AutoSignature self-approval protocol (cloud escalation always human-gated); emit FR-EVT-GOV-*.

9. **Dev experience and ADRs**  
   - One-command dev startup MUST include local model runtime (or a mock) and sample jobs.  
   - Create initial Architecture Decision Records (ADRs) for key choices:
     - Runtime selection.  
     - DB layout for jobs and Flight Recorder.  
     - Capability model shape.  

10. **Security, resource, and UX bridges for mechanical work**  
    - Safety gates: wire `Guard` (secret/safety scan), `Container` (isolated exec), and `Quota` (resource limits) for mechanical/terminal jobs.  
    - Observability: expose `Profiler`/`Monitor` system metrics tied to job/session identifiers.  
    - Devops: route `Repo`/`Formatter`/minimal `Deploy` through the same capability/FR pathways used for other jobs.  
    - UX bridges: expose `Clipboard`/`Notifier` actions only with explicit capability/consent.
    - [ADD v02.136] Unified Tool Surface baseline (Â§6.0.2): route **all** tool invocations (local tool calling + MCP + MEX engines) through **Tool Gate** for schema validation, capability enforcement, payload limits (32KB rule), secret redaction, idempotency keys, and FR-EVT-007 (ToolCallEvent) logging.


11. **MCP skeleton and Gate (Target 1 + job/log plumbing)**  
    - [ADD v02.136] MCP MUST NOT introduce a second tool schema. MCP tool discovery/schemas are generated from the **Tool Registry** (HTC v1.0, Â§6.0.2) and every `tools/call` is enforced by **Tool Gate**.
    - Implement a minimal MCP client stack in the Rust coordinator, even if only exercised against a local stub server:
      - JSON-RPC transport and tool/resource discovery for at least one MCP server.  
      - Connection lifecycle tied to workspace/session where appropriate.  
    - Implement the MCP **Gate** interceptor (Section 11.3.2) as middleware around the MCP client:
      - Intercept `tools/call` requests, attach `job_id` / workflow run IDs and capability metadata.  
      - Enforce basic consent decisions and log them into Flight Recorder.  
      - Capture and log `tools/call` and `sampling/createMessage` traffic end-to-end, even when using a stub MCP server.  
    - Extend the AI Job Model to support MCP jobs:
      - Add a `transport_kind = "mcp"` discriminator and fields for `mcp_server_id` and `tool_name` where applicable.  
      - Ensure at least one test job profile uses MCP end-to-end (job â†’ MCP call â†’ response â†’ logs).  
    - Ensure Flight Recorder can represent MCP events using the canonical event shape in Section 11.3:
      - At least one MCP request/response path visible in the Flight Recorder UI.  
      - Clear correlation between a UI action, the AI job, and the MCP tool call(s) it triggered.  
12. **Calendar (local-only) as a Flight Recorder lens (no external sync)**  
   - Implement minimal **CalendarEvent** storage and rendering for a local-only calendar surface (manual create/edit local events).  
   - Implement **time-range selection** that queries Flight Recorder by interval overlap (ActivitySpans/SessionSpans) and renders an event â€œActivityâ€ tab (sessions, jobs, tool calls, models).  
   - Calendar writes MUST be applied via a local-target `calendar_sync` mechanical job (patch-set discipline + capability gate); UI remains read-only over authoritative state.  
   - Capabilities: `CALENDAR_READ_BASIC` / `CALENDAR_READ_DETAILS` for viewing; `CALENDAR_WRITE_LOCAL` for local edits.  



13. **[ADD v02.44] OSS governance baseline (build/release enforcement)**  
   - Enforce Â§11.7.4 requirements: every shipped OSS component MUST be present in the OSS Component Register with license + integration_mode + pinning policy.
   - Enforce the copyleft isolation rule: GPL/AGPL components MUST be `product_managed_process` or explicit `operator_configured_adapter` (never linked into the app binary).
   - Gate release builds on register completeness + policy compliance.

#### 11.7.5 Supply Chain Mechanical Gates (MEX v1.2)
- **Engine IDs**:
  - `engine.supply_chain.vuln`: Wraps `cargo-audit` / `npm audit` / `osv-scanner`.
  - `engine.supply_chain.sbom`: Generates CycloneDX / SPDX via `syft`.
  - `engine.supply_chain.license`: Wraps `scancode-toolkit` or `cargo-deny`.
- **Capability Requirements**: All supply-chain engines require `proc.exec` for their respective binaries and `fs.read:inputs`.
- **Artifact Schemas**:
  - `SupplyChainReport { kind: Vuln | SBOM | License, engine_version: String, timestamp: DateTime, findings: JSON }`.
- **Governance**: Any HIGH severity vulnerability or UNKNOWN license found during a `release` build MUST be emitted as a `BLOCK` problem in the diagnostics registry.

14. **[ADD v02.44] Deliverables PDF pipeline (MVP)**  
   - Implement `creative.deliverables.pdf_packaging` as a first-class Job path: Typst render + qpdf packaging.
      - Store output artifacts with provenance and deterministic output checks (byte-stable where feasible; otherwise stable hash policy).
      - **Implementation Note:** See Â§11.10.1 for binary resolution and job constraints.
   
   15. **[ADD v02.52] Bundle Export Framework v0 (MVP)**
      - **Debug Bundle export**: implement end-to-end exactly as specified in the Master Specâ€™s Debug Bundle section (no edits).
      - **Workspace Bundle export v0**: backup/transfer/fixture export for docs/canvases/tables + raw assets (when present).
   
   16. **[ADD v02.53] AI Rewrite UI Primitives (Human-in-the-Loop)**
      - Implement `DOC_REWRITE` workflow with "Diff" view and "Accept/Reject" gating.
      - Enforce "No Silent Edits" invariant via UI and Backend Gate.
      - Log rejected variations to Flight Recorder.
   
   
17. **[ADD v02.79] Photo Studio v0 (skeleton surface + governance wiring)**  
   - [ADD v02.79] Import JPEG/PNG/TIFF as Assets; generate thumbnails/previews as artifacts (no binaries in prompts/logs).  
   - [ADD v02.79] Minimal "edit recipe" placeholder stored as versioned sidecar (even if only exposure/WB placeholders).  
   - [ADD v02.79] Export via governed job path (artifact + export record; no direct file mutation).  
18. **[ADD v02.101] Spec Router and governance session log (MVP)**  
   - Implement `spec_router` job_kind and SpecRouterJobProfile with policy-bound mode selection.  
   - Emit SpecIntent and SpecRouterDecision artifacts with `capability_registry_version`.  
   - Auto-create or update Task Board and Work Packet entries for GOV_STRICT/GOV_STANDARD and append Spec Session Log entries.  
   - Enforce git-only safety commit behavior for git workflows; non-git workflows must not attempt a commit.  
   - [ADD v02.128] Implement **Spec Creation System v2.2.1** routes as **command-driven** intents (model never guesses): `/spec new`, `/spec extend`, `/spec refine`, `/spec check`, `/task` (see Â§2.6.8.13).  
   - [ADD v02.128] Persist and deterministically update `SPEC_INDEX.yaml` metadata for spec creation/refinement (tracks spec-creation-system version separately from Master Spec version; see Â§2.6.8.13).  
   - [ADD v02.128] Validate/enforce **Universal IDs + requirement grammar** on produced specs; validation failures MUST block Work Packet activation in GOV_STRICT/GOV_STANDARD and surface in Problems + Spec Session Log.  
   - [ADD v02.128] Run **overlap/conflict detection** before activation; emit a deterministic conflict report artifact and link it from SpecRouterDecision + Spec Session Log (block activation on hard conflicts in GOV_STRICT).  
   - [ADD v02.128] `/spec check` MUST run the **full rubric workflow** (rubric + second model + red-team pass) and emit a check report artifact (hash-linked, provenance-complete) visible in Operator Consoles.  

13. **[ADD v02.36] ACE Runtime (MVP) + Validator Pack (CI-gated)**  
   - For every AI job: emit and persist **ContextPlan** and per-call **ContextSnapshot** artifacts.  
   - Enforce the runtime validators (see Â§2.6.6.7.11) on every job; violations fail the job with normalized diagnostics.  
   - Debug Bundle export includes: ContextPlan, ContextSnapshots, validator outcomes, and evidence refs used.
   - [ADD v02.138] Front End Memory System (FEMS) v0: compile and inject bounded `MemoryPack` (â‰¤500 tokens) per call; generate `MemoryWriteProposal`s (no implicit writes); route procedural memory through DCC review; emit `FR-EVT-MEM-*`; add FEMS-EVAL-001.


14. **[ADD v02.36] Terminal LAW (minimal slice) promoted to MUST**
   - Terminal command execution MUST NOT bypass capabilities/consent, Workflow Engine, Gate, or Flight Recorder.
   - Every terminal run is bound to `job_id` / `workflow_id` and records scrubbed context + artifact references as provenance.
   - **[ADD v02.63] ModelProfile/Routing/SafetyProfile clarity:** Phase 1 runtime integration MUST define a concrete profile schema (id, role, safety policy, routing notes) for models used in MVP.

15. **[ADD v02.36] Capability single-source-of-truth + unknown-capability validator**  
   - Resolve all capability declarations via job profiles (`capability_profile_id`) into a single normalized set used by Gate + UI.  
   - Unknown/undeclared capability requests fail fast and surface an explanation in Problems/Evidence.


16. **[ADD v02.38] Canvas Typography + Font Packs (Design Pack + Font Registry)**  
   - Bundle offline font packs in app resources and ship licensing artifacts (per-font license files + THIRD_PARTY_NOTICES).  
   - Implement backend-owned Font Registry (bootstrap pack, rebuild manifest, list families, import/remove fonts).  
   - Deterministic font loading via `FontFace` / `document.fonts.ready`; no â€œflash of fallbackâ€ on first render.  
   - Sanitize font names and file paths to prevent CSS injection; UI never crawls font directories directly.  
   - Canvas text objects can select bundled fonts; export (PNG/SVG) preserves selected font.
   - **Implementation Note:** See Â§11.10.2 for runtime root and CSP policy.

17. **[ADD v02.42] Iterative Deepening (snippet-first) â€” MVP policy scaffolding**  
   - Enforce snippet-first retrieval policy for any retrieval-capable job in Phase 1 (local workspace search, MCP reads): start with bounded snippets only; no full-document/page injection paths. (See Â§2.6.6.7.5.2.)  
   - Implement SEARCH â†’ READ separation in adapters (stubs acceptable in Phase 1): `search(query) -> snippets` and bounded `read(section_selector) -> excerpt`.  
   - Emit and log EvidenceSnippets with (minimum) `fetch_depth = snippet`, `trust_class`, a resolvable citation/SourceRef, and a 1â€“2 line relevance rationale; enforce per-step retrieval budgets and anti-context-rot rules (dedupe, exclude tool logs).  

18. **[ADD v02.52] Retrieval Correctness & Efficiency (ACE-RAG-001) â€” Phase 1 plumbing**
   - Emit and persist `QueryPlan` and `RetrievalTrace` for every retrieval-backed model call; link both to the `ContextSnapshot` / `PromptEnvelope`.
   - Implement deterministic `normalized_query_hash` (sha256 of normalized query text) and record it in `RetrievalTrace`.
   - Compute and record `CacheKey` for cacheable stages (even if cache is initially a stub); log cache hit/miss per stage.
   - Enforce hard budgets at runtime:
     - `RetrievalBudgetGuard` (evidence tokens/snippet counts/read caps; deterministic truncation with flags).
     - `CacheKeyGuard` (strict mode requires cache key computation + logging).
   - Add a minimal Semantic Catalog registry (built-in) so routing does not depend on â€œLLM guessingâ€ store/tool names.
   - Operator Consoles MUST deep-link: Job â†’ Model Call â†’ QueryPlan/Trace â†’ Evidence items (SourceRefs/ArtifactHandles) without opening raw documents by default.


19. **[ADD v02.67] Atelier Lens Runtime v0.1 (Role claiming + dual-contract extraction)**
   - Implement `ATELIER_CLAIM` as a mechanical job (capability-gated; logged) that emits:
     - `RoleScore[]` (dense distribution over all registered roles)
     - `RoleClaim[]` (thresholded multi-claim set used to schedule deep passes)
     - `RoleGlance[]` (cheap all-roles glance; **SHOULD** cover every role; may record `found=false` without blocking)
   - Implement a versioned `AtelierRoleSpec` registry and enforce dual contract ids:
     - `ROLE:<role_id>:X:<ver>` (extraction) â†’ `RoleDescriptorBundle`
     - `ROLE:<role_id>:C:<ver>` (creative) â†’ `RoleDeliverableBundle`
   - Implement Lens job profiles (all through Workflow Engine + Flight Recorder):
     - `ATELIER_ROLE_EXTRACT`, `ATELIER_ROLE_COMPOSE`, `ATELIER_STATE_MERGE`, `ATELIER_GRAPH_SOLVE`, `ATELIER_CONCEPT_RECIPE`
   - Evidence discipline: every claimed field MUST have `EvidenceRef` (bbox/page/span/time-range) and must pass `ATELIER-LENS-VAL-003` (missing evidence is FAIL).
   - MVP role set MUST include at least one Finishing role contract (Editor or Color) alongside pre/prod roles.
   - Wire Lens validators `ATELIER-LENS-VAL-007..011` as required gates for Lens runs (merge determinism/conflict accounting, recipe validity, DAG validity, dependency completeness).

20. **[ADD v02.68] Mechanical Extension v1.2 runtime contract (MEX) â€” Phase 1 foundations**
   - Implement Engine PlannedOperation (`schema_version=poe-1.0`) + EngineResult envelopes; validate with `G-SCHEMA`.
   - Implement the required gate pipeline for engine jobs: `G-CAP`, `G-INTEGRITY`, `G-BUDGET`, `G-PROVENANCE`, `G-DET`; log every decision/outcome to Flight Recorder and surface denials in Problems.
   - Implement the engine registry loader (`mechanical_engines.json`) and adapter resolution; capabilities are default-deny and must be explicitly granted/recorded.
   - Implement **Conformance Harness v0** and require at least **3 engines** to pass conformance (recommended: Context/Sandbox/Wrangler or equivalents) before enabling additional engines.
   - Enforce artifact-first I/O: any payload >32KB uses artifact handles; outputs are artifacts with SHA-256 + sidecar provenance manifests; no direct filesystem bypass (materialize-only).
   - Canonical references: Â§6.3.0 + Â§11.8.
- [ADD v02.79] Import a small photo set â†’ open Photo Studio â†’ apply a minimal recipe stub â†’ export a derivative â†’ inspect provenance in Job History / Flight Recorder.


21. **[ADD v02.102] Phase 1 closure: storage backend portability work packets (CX-DBP-030)**
   - Phase 1 cannot close until the following are complete and validated (see Section 2.3.13.5 [CX-DBP-030]):
     - WP-1-Storage-Abstraction-Layer
     - WP-1-AppState-Refactoring
     - WP-1-Migration-Framework
     - WP-1-Dual-Backend-Tests

22. **[ADD v02.102] CapabilityRegistry single source of truth (WP-1-Capability-SSoT)**
   - Ensure CapabilityRegistry resolves scoped requests against axis-level grants and produces deterministic allow/deny results for Gate + Spec Router (see Section 11.1.6 validator requirement).

23. **[ADD v02.102] Global Silent Edit Guard (WP-1-Global-Silent-Edit-Guard)**
   - Implement StorageGuard validation: AI writes must include job/workflow context and persist MutationMetadata; reject silent AI edits with `HSK-403-SILENT-EDIT` (see Section 2.9.2).

24. **[ADD v02.102] Phase 1 final gap closure details (Section 11.10)**
   - `creative.deliverables.pdf_packaging` discovers `typst` and `qpdf` via PATH or env vars `HANDSHAKE_TYPST_BIN` / `HANDSHAKE_QPDF_BIN`.
   - Fonts runtime root is `{APP_DATA}/fonts/` and Tauri `asset:` protocol is restricted to that directory; bootstrap copies bundled font pack(s) to `{APP_DATA}/fonts/bundled/`.
   - On startup, detect Handshake-native ModelRuntime capabilities and enable the local client by default when present. Ollama detection may exist only as an explicit ExternalEngineImport compatibility path, never as the default.

25. **[ADD v02.103] Response Behavior Contract (Diary ANS-001)**
   - Implement the Response Behavior Contract (Section 2.7) for governed assistant responses: intent confirmation, mode context, operation plan, proactive surfacing, and next steps.
   - Work modes (Strict/Free/Fasttrack/Brainstorm/Data) must deterministically constrain allowed operations and determinism requirements.
   - [ADD v02.121] Persist frontend interactive chat sessions to `{APP_DATA}/sessions/<session_id>/chat.jsonl` (Â§2.10.4) including raw chat text and ANS-001 payload per frontend assistant message.
   - [ADD v02.121] UI: ANS-001 is hidden inline by default with per-message expand + global show-inline toggle, and is available in a side-panel timeline viewer (Â§2.7.1.7).

- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - Runtime modes: DOCS_ONLY vs AI_ENABLED; enforce min_ready_models=1 in AI_ENABLED.  
  - Multi-model orchestration primitives: ExecutionMode, model readiness, swap/escalation.  
  - File-scope lock enforcement for concurrent WPs/MTs (no overlapping IN_SCOPE_PATHS).  
  - Work Profile routing: ParameterClass + â€œlargest-firstâ€ selection + model performance telemetry scoring.  
  - RoleExecutionIdentity logging per output (role, model_id, backend, parameter_class, cloud_strength, session_id).  
  - Role Mailbox persistence taxonomy (MailboxKind) and non-authority boundary.  
  - HSK_STATUS single-line lifecycle marker; shown after gate output.  
  - Softblock/failstate code registry (CX-MM-xxx) for model readiness, lock conflicts, swap failures.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - DCC surface stub in UI (kanban-first), with panels: **Work Packets**, **Workspaces (worktrees)**, **Sessions**, **Approvals**, **VCS** (status/diff), **Search**.  
  - Workspace registry: list/add/open/close worktrees; import existing worktrees; link workspace â†’ `wp_id`/`mt_id` (no authority drift; Locus remains canonical).  
  - Execution Session Manager: show active sessions (role/model/backend), workspace binding, capability grants; deep-link to Job History + Flight Recorder timeline.  
  - Approval Inbox: render pending capability requests from the Workflow Engine; support approve-once / approve-for-job / approve-for-workspace / deny; log all decisions to Flight Recorder.  
  - VCS review loop: show `version.status` + `version.diff`; commit flow uses `version.commit(paths[])` with commit message as an artifact; **no implicit staging**; dangerous ops (reset/clean/rebase) require same-turn explicit approval.  
  - Objective Anchor Store (minimal): create/view anchors and handoffs linked to `wp_id`/`mt_id`; anchors are **non-authoritative** and MUST NOT override Locus status.  
  - Storage foundation: ship `.handshake/workspace.json` schema v1.0 and `devcc.db` schema v1; both local-first; no secrets in committed artifacts; default ignore patterns documented.  
  - Conversation timeline (Phase A baseline): ingest Handshake-native conversations (roles + Flight Recorder events) into DCC timeline; adapter contract stubbed for later external sources.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - Runtime contract hook: reserve and accept `settings.exec_policy` (optional) with deterministic downgrade semantics (requested vs effective) (Â§2.5.2.1, Â§4.5.5).  
  - Work Profile hook: per-role compute presets + separate approximate knob with waiver requirement (`hsk.work_profile@0.6`) (Â§4.3.7.5, Waiver Protocol [CX-573F]).  
  - Observability: Flight Recorder event `llm_exec_policy` (FR-EVT-LLM-EXEC-001) + referenced trace artifact format `hsk.layerwise_trace@0.1` (Â§11.5.11).  
  - UI/config surface: operator can select per-role `speed_preset` (standard / fast_exact / fast_approx) and cannot enable approximate without a waiver reference.  
  - Default posture remains **standard exact**; no planner auto-enables approximation.



26. **Loom MVP (Heaper-style library surface)** [ADD v02.130]  
   - Implement the LoomBlock entity (Â§2.2.1.14) and LoomEdge relational model (Â§2.3.7.1) with CRDT-safe UUID references (rename-stable).
   - Implement the four Loom views (Â§7.1.4.3; Â§10.12): **All**, **Unlinked**, **Sorted**, **Pins** with grid/list toggle and required filters.
   - Implement file import into Loom (folder drag/drop + file picker) with SHA-256 dedup (**FR-EVT-LOOM-006**) and stable identity independent of filename (**[LM-BLOCK-004]**).
   - Implement Tier-1 preview generation (thumbnails, lightweight proxies where needed) as Mechanical jobs; attach `thumbnail_asset_id` / `proxy_asset_id` to LoomBlocks.
   - Integrate inline @mentions + #tags in the Rich Text Editor (Â§7.1.1.7) and ensure tag targets are `TAG_HUB` LoomBlocks.
   - Provide LoomBlock detail view with backlinks panel (DerivedContent) and context snippets (**[LM-BACK-001]**..**[LM-BACK-003]**).
   - Implement Tier-1 Loom search (SQLite FTS over `derived.full_text_index`) with basic facets (content_type, mime, date, tag, mention).
   - Emit and surface FR-EVT-LOOM-001..012 (Â§11.5.12) in Operator Consoles / Job History.

27. **Handshake Stage MVP (governed browser + Stage Apps)** [ADD v02.131]  
   - Implement the Stage Host surface: session + tab model with strict origin isolation (External Web vs Stage Apps) and per-session cookie/storage boundaries (no bleed).  
   - Implement `handshake-stage://` scheme loader for Stage Apps with bundle integrity (SHA-256) + CSP defaults; Stage Apps MUST NOT be able to navigate to arbitrary `http(s)` without explicit user action and host allowlist enforcement.  
   - Implement Stage Bridge API (request/response + events) with capability-gated methods: `stage.runtime.getContext`, `stage.jobs.enqueue`, `stage.workspace.createDocumentFromArtifact`, `stage.workspace.applyPatchSet`, `stage.ui.requestApproval`, `stage.ui.notify` (Phase 1 subset acceptable per Stage spec).  
   - Implement evidence-grade web capture workflow `stage.capture_webpage.v1` (Archivist) producing `artifact.snapshot` bundles (HTML + assets + screenshots) and provenance manifests; enforce artifact-first I/O (>32KB params via input artifacts).  
   - Implement selection clipping `stage.clip_selection.v1` to convert selected DOM ranges â†’ `artifact.clip` (markdown + source selectors) with traceable links to the originating snapshot.  
   - Provide PDF viewing + import controller: attach PDFs as artifacts and enqueue `stage.import_pdf.v1` producing a document stub; **Docling-backed structured conversion is Phase 2** (Â§7.6.4).  
   - Deliver Stage Apps Phase 1 set: `com.handshake.stage.clip` (web â†’ doc/clip), `com.handshake.stage.pdf_import` (PDF view/import controller), and `com.handshake.stage.playground` (prompt/spec playground with controlled job spawn).  
   - Deliver 3D Mechanical Assist Pack Phase 1 slice: `stage.3d.import_gltf.v1`, `stage.3d.extract_scene_ir.v1`, `stage.3d.validate_gltf.v1`; add a read-only 3D viewport/inspector to render `artifact.3d_asset` and display `artifact.scene_ir` + validator/physics reports.  
   - Ship a Stage security harness that asserts: bridge injection only for Stage Apps; external web cannot call bridge; private-network navigation is denied by policy; and every privileged Stage Bridge call is logged in Flight Recorder with allow/deny + capability IDs.  


28. **Multi-Session Orchestration: ModelSession + Scheduler:** Add `ModelSession` as the persisted unit of multi-turn orchestration; add `model_run` job_kind and a session scheduler with queueing, cancellation, and concurrency limits.
29. **ModelSession persistence layer:** Store sessions + message artifacts in local workspace DB (Phase 1) with explicit content-hash discipline.
30. **Session spawn contract (OpenClaw pattern):** Implement `SessionSpawnRequest` + `SessionSpawnResponse`, depth limits, per-session spawn caps, and Role Mailbox announce-back (SessionAnnounceBack) with summary artifacts.
31. **Session-scoped capability tokens:** Add `capability_token_ids` on ModelSession and enforce deny-by-default session-scoped capability intersection in Tool Gate.
32. **Tool calling + structured output adapters:** Implement provider adapters that translate the Unified Tool Surface registry into provider tool schemas and back (no parallel ToolDefinition schema).
33. **Workspace safety boundaries for parallel writes:** Implement worktree isolation or file locking policy for concurrent sessions on the same workspace, with deterministic conflict handling.
34. **Crash recovery / resume:** Add session checkpointing and idempotent recovery flow for interrupted `model_run` jobs.
35. **DCC multi-session steering panel:** Display session list, state machine, spawn tree, cost/budget per session, and controls (pause/resume/cancel).
36. **Flight Recorder coverage:** Register and emit FR-EVT-SESS-*, FR-EVT-SESS-SCHED-*, FR-EVT-SESS-SPAWN-*; add `model_session_id` correlation to base event schema.
37. **Session observability bindings:** Implement `ModelSessionSpan` creation/closure per ModelSession lifecycle and ActivitySpan linkage for `model_run` and tool calls; session-wide queries MUST work via `model_session_id` even without spans.

**Vertical slice**
- **Core loop**
  - Start the app and open a sample document.
  - Select text and trigger â€œRewrite selectionâ€.
  - See the updated text in the document.
  - Open Job History and locate the corresponding job with correct status and metadata.
  - Inspect logs and traces that show the model call, workflow execution, and any errors.
  - [ADD v02.136] Trigger one governed tool call (e.g., `engine.context.search` or `engine.sandbox.exec`) and verify: Tool Gate decision recorded, FR-EVT-007 (ToolCallEvent) visible, artifacts referenced (no large blobs in context), and the call is linked to the originating job/workflow.
  - Open a Canvas and create a text object.
  - Choose a bundled font family; ensure it renders deterministically (no fallback flash).
  - Save, restart, reopen; typography selection persists.
  - Export Canvas to PNG/SVG; exported result preserves the chosen font.
  - Export a deliverable PDF (Typst + qpdf); exported result is reproducible (byte-stable or stable hash policy).
  - [ADD v02.52] Trigger "Ask about this document" (RAG-aware Q&A) and verify Evidence view exposes QueryPlan + RetrievalTrace ids/hashes and bounded spans (with truncation flags if budgets hit).
  - [ADD v02.52] Export a Workspace Bundle for a non-trivial workspace and verify: manifest + doc/canvas/table snapshots + export report.
  - [ADD v02.52] Export a Debug Bundle for one AI job and verify required files + SAFE_DEFAULT redaction mode.
  - [ADD v02.101] Run Spec Router on a prompt, verify SpecIntent/SpecRouterDecision artifacts, Task Board + Work Packet creation, and a Spec Session Log entry; if the workspace is git-backed, verify safety commit behavior.
  - [ADD v02.128] Run `/spec new`, `/spec extend`, `/spec refine`, and `/spec check` end-to-end and verify: command-driven routing (no heuristic guessing), `SPEC_INDEX.yaml` updated deterministically, Universal IDs/requirement grammar validation, overlap detection report emitted, and `/spec check` rubric+2nd-model+red-team report linked in Spec Session Log.

- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**  
  Run WP-A and WP-B in parallel on two models; WP-B attempts overlap â†’ blocks; WP-A completes; operator swaps WP-B to larger model after an escalation failstate.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**  
  Run one WP end-to-end using DCC: create/link worktree â†’ run job â†’ approval prompt â†’ review diff â†’ commit â†’ mark MT done; confirm Task Board sync and Flight Recorder evidence.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**  
  Operator selects `fast_exact` for `worker`, runs a WP/MT, and sees requested vs effective policy in FR + UI; then toggles approximate with a waiver and sees `llm_exec_policy` + trace reference emitted.


- **Loom loop** [ADD v02.130]
  - Import a folder containing mixed media (images, PDFs, audio, video) into Loom.
  - Verify dedup: importing the same file twice routes to the existing LoomBlock (no duplicate).
  - Browse **All** in grid mode; open an item; add a short annotation; create an @mention and a #tag.
  - Confirm the item disappears from **Unlinked** immediately after the first link/tag.
  - Open the #tag hub and confirm backlinks show context snippets.
  - Run Loom search (FTS) with at least one filter facet; open a result and confirm Flight Recorder events exist for import, preview generation, edge creation, view query, and search.

- **Stage loop** [ADD v02.131]
  - Open Handshake Stage and start a new isolated session.
  - Navigate to an external web page; capture it via `stage.capture_webpage.v1`; verify an `artifact.snapshot` bundle is stored with hashes + provenance.
  - Clip a selection; verify `artifact.clip` links back to snapshot selectors; create a workspace document from the clip via Stage Bridge (`stage.workspace.createDocumentFromArtifact`) with approvals logged.
  - Open a PDF in Stage; run `stage.import_pdf.v1`; verify PDF bytes are preserved as an artifact and the resulting document stub links to it.
  - Import a glTF/GLB; run `stage.3d.import_gltf.v1` + `stage.3d.validate_gltf.v1`; open the 3D viewport and view `artifact.scene_ir` + validator reports.
  - Confirm Stage Bridge calls are capability-gated and show allow/deny decisions + linked jobs in Operator Consoles / Flight Recorder.


**Key risks addressed in Phase 1**
- [ADD v02.130] Loom integrity/performance risks: UUID-stable inline tokens (no text-based links), anchor drift during edits, dedup false positives/negatives, and preview-generation throughput (Tierâ€‘1 thumbnails) must not degrade core UI responsiveness.
- [ADD v02.131] Stage security/evidence risks: origin isolation bugs (External Web â†” Stage Apps), session bleed (cookies/storage), private-network access, and capture-evidence integrity drift (missing hashes/provenance) must be prevented by policy enforcement + security harness + always-on Flight Recorder logging.
- [ADD v02.136] Tool surface drift + prompt-injection risk: if local tools, MCP tools, and mechanical engines have **different** schemas/logging, agents will find bypass paths and tool outputs may smuggle instructions. Mitigation: single Tool Registry + Tool Gate, strict payload caps, artifact-first I/O, and mandatory FR-EVT-007 (ToolCallEvent) logging.
- [ADD v02.138] Memory poisoning / drift risk: untrusted session text or tool outputs promoted into procedural memory (or oversized `MemoryPack`s) can degrade correctness and increase drift vectors. Mitigation: FEMS write gates + human review for procedural memory, hard pack budgets (â‰¤500 tokens), and replay-grade logging (`FR-EVT-MEM-*`).



- [ADD v02.47] Prevent MVP scope creep: charts/dashboards/decks are deferred to Phase 2+ (no partial implementation in Phase 1).

- Calendar/time-lens introduces hidden writes or breaks deterministic provenance (must remain patch-set + logged + capability-gated).


- AI Job Model and Workflow Engine are too complex or too weak for real usage.  
- Observability (Flight Recorder, metrics, traces) is not wired end-to-end.
- Operator cannot produce deterministic bug evidence (Debug Bundle + Problems) â†’ LLM coding loop stalls.
  
- Capability and consent models are unclear or easily bypassed.  
- Secret/resource leakage or runaway jobs if Guard/Container/Quota are absent or unenforced.
- MCP Gate and MCP-ready job/log plumbing are bolted on too late, forcing breaking changes to job/log schemas or inconsistent consent/logging across tools.

- [ADD v02.101] Prompt-to-spec routing is not policy-bound, causing governance drift or inconsistent artifacts; mitigated by Spec Router policy schema and session log.
- [ADD v02.139] Spec hallucination / non-executable spec risk: models invent non-existent capabilities/tools or omit concrete validation hooks; mitigated by CapabilitySnapshot allowlist enforcement + deterministic SpecPromptPack compilation + SpecLint preflight (SPEC-VAL-*).
- [ADD v02.128] Spec Creation System gaps (missing command routing, rubric discipline, overlap detection) cause inconsistent specs and collisions across WPs; mitigated by command-driven routes + mandatory `/spec check` + deterministic overlap/conflict reports.
- [ADD v02.128] Missing Universal IDs / requirement grammar causes ambiguous references and audit failures; mitigated by v2.2.1 Universal ID system + validators.
- [ADD v02.101] Safety commit logic applied outside git workflows can destroy non-git state; mitigated by explicit VersionControl gating and policy rules.


- [ADD v02.36] "Auditable AI" is non-real without enforced ContextPlan/ContextSnapshot + runtime validators (not just logging).
- [ADD v02.36] Terminal is a capability-bypass vector unless fully routed through Gate/Workflow/Flight Recorder (Terminal LAW).
- [ADD v02.38] Canvas editor choice (Excalidraw) may constrain deterministic font loading/text editing; validate compatibility with Â§10.6 before implementing typography acceptance criteria.


- [ADD v02.44] OSS license posture drift: accidental in-process use of GPL/AGPL, or missing/incorrect OSS Register entries, undermines auditability and distribution.
- [ADD v02.44] Mixed-license tools (e.g., ExifTool dual license; Czkawka with GPL sub-app) require strict `product_managed_process` posture and explicit capability gating.
- [ADD v02.49] Unmanaged OSS/tool outputs (random files) create untraceable side effects and break reproducibility; mitigated by manifest + SHA-256 + materialize-only semantics.
- [ADD v02.49] Disk bloat / cache drift from derived outputs; mitigated by TTL + pinning + deterministic GC with visible reports.

- [ADD v02.52] Retrieval remains opaque / non-replayable (answers cannot be explained or reproduced); mitigated by mandatory QueryPlan + RetrievalTrace + deterministic tie-breaks and persisted selection inputs.
- [ADD v02.52] Token budgets silently drift upward (context bloat, slower answers, worse correctness); mitigated by hard BudgetGuard enforcement + deterministic truncation flags + CI fixtures.
- [ADD v02.52] No redaction-safe evidence packet for LLM coders/validators (Debug Bundle).
- [ADD v02.52] Data lock-in / inability to back up workspace state early (Workspace Bundle).
- [ADD v02.52] Accidental leakage through export (exportable=false enforcement + policy gating).


- [ADD v02.67] Atelier role overlap/contradictions can produce nondeterministic outputs unless merge/arbitration is explicit; mitigated by `SceneState` + `ConflictSet` + `ATELIER_STATE_MERGE` and validators `ATELIER-LENS-VAL-007/008`.
- [ADD v02.67] Conceptual Mode vectors are not replayable/auditable if they remain UI-only; mitigated by typed `ConceptRecipe` and `ATELIER-LENS-VAL-009`.
- [ADD v02.67] Nested Production remains prose without enforceable dependencies; mitigated by `AtelierProductionGraph` + `ATELIER_GRAPH_SOLVE` and validators `ATELIER-LENS-VAL-010/011`.
- [ADD v02.67] â€œEveryone finds somethingâ€ can degrade relevance and blow up compute; mitigated by `RoleGlance` (cheap, non-blocking) + thresholded `RoleClaim` (top-k deep passes) + explicit per-run budgets.


- [ADD v02.68] Mechanical jobs become an ungoverned â€œescape hatchâ€ (bypass, side effects, missing provenance); mitigated by v1.2 envelopes + required gates + registry + conformance harness, with denials visible in Problems/Flight Recorder.
- [ADD v02.68] Unbounded mechanical outputs (logs/large blobs) pollute context and break artifact discipline; mitigated by artifact-first I/O (>32KB rule) + output caps + G-BUDGET/G-PROVENANCE enforcement.
- [ADD v02.79] Scope explosion (Lightroom/Affinity-class) â†’ enforce Phase 1 boundary: â€œskeleton only; no RAW/masks/layers/AIâ€.
- [ADD v02.79] UI bypassing job runtime â†’ all Photo Studio actions MUST enqueue jobs (â€œsingle execution authorityâ€).


- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - Conflicting edits across parallel WPs (prevented by strict file-scope locks).  
  - Governance bypass via side channels (prevented by non-authority role mailbox + canonical artifact rules).  
  - Opaque multi-model state (solved by HSK_STATUS + FR correlation).

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - Governance bypass via â€œUI direct execâ€ (prevented by forcing all actions through jobs + approvals + Flight Recorder).  
  - Lost context and brittle handoffs between sessions/models (reduced via anchors + handoff records + session identity).  
  - Parallel work collisions across WPs/MTs (mitigated via worktree-per-WP discipline + Locus lock semantics + visibility).

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - Prevent silent quality regressions (approximate execution must be explicit + waived).  
  - Prevent â€œunknown computeâ€ in audits (requested vs effective policy always logged).  
  - Prevent privacy leaks from high-volume traces (no token IDs; no raw text by default).
- [ADD v02.137] Uncontrolled parallel model spawning (budget blow-ups / runaway sessions); mitigated by spawn limits + per-session budget + scheduler backpressure + kill switch.

**Acceptance criteria**
- [ADD v02.130] Loom: importing files creates LoomBlocks; dedup prevents duplicates; All/Unlinked/Sorted/Pins operate correctly; @mentions/#tags create edges; backlinks update with snippets; Tierâ€‘1 search works with facets; FR-EVT-LOOM-* are emitted and visible in Operator Consoles.
- [ADD v02.131] Stage: External Web and Stage Apps run in isolated sessions/tabs (no cookie/storage bleed); Stage Bridge is injected only on `handshake-stage://` and denies calls from External Web; every Bridge allow/deny is visible in Flight Recorder/Operator Consoles.
- [ADD v02.131] Stage capture/import: `stage.capture_webpage.v1` produces `artifact.snapshot` bundles with manifests + SHA-256; `stage.clip_selection.v1` produces `artifact.clip` linked to snapshot selectors; PDF import preserves bytes as artifacts and produces a doc stub; glTF import/validate produces `artifact.scene_ir` + validator reports; all outputs survive restart and are discoverable.
- [ADD v02.139] Spec Router: compiled PromptEnvelope hashes + SpecPromptPack id/sha + CapabilitySnapshot ref are emitted for every spec job; SpecLint gate (G-SPECLINT) blocks progression on failure and produces a SpecLintReport artifact linked in Job History + Spec Session Log; SpecLint runs in CI for `.SPEC/` outputs (v2.2.1) and for GOV_STANDARD/STRICT Feature/Workflow/Integration specs.
- [ADD v02.142] Runtime visibility seed: Appendix 12.3 includes capability slices/runtime visibility rows for Calendar temporal correlation, Calendar orchestrated mutation, unified local/cloud governed tool calling, Locus execution correlation, Loom retrieval library, and Stage capture/import pipeline; Appendix 12.6 links those rows into force-multiplier edges visible to operator/runtime surfaces.
- [ADD v02.143] Primitive index seed: Appendix 12.4 is normalized to coverage-driven rows and includes concrete runtime/job/tool/frontend/operator primitives plus stub-linked gap tracking for Calendar, Locus, Loom, AI-ready retrieval, Spec Router, Stage-adjacent media flows, and unresolved Mail/Studio runtime coverage.
- [ADD v02.144] Second-pass coverage sweep: Appendix 12 explicitly covers Canvas, Docs & Sheets, Project Brain, Thinking Pipeline, Context Packs, Semantic Catalog, Skill Bank, ASR, Charts & Dashboards, Presentations / Decks, Studio, and richer Mail / DCC runtime projection; unresolved embodiments are linked to detailed stubs before matrix expansion continues.
- [ADD v02.145] Third-pass coverage sweep: Appendix 12 explicitly covers Model Session Orchestration, Cloud Escalation Consent, MEX Runtime, and the typed runtime/export/filter/session contracts behind AI Job Model, Workflow Engine, Flight Recorder, Diagnostics, Operator Consoles, and Unified Tool Surface; unresolved embodiments remain stub-linked before wider matrix expansion continues.
- [ADD v02.146] Deepening sweep: Appendix 12 explicitly covers AI-ready index artifacts/status enums, AI Job and Flight Recorder UI/operator surfaces, Role Mailbox export/transcription contracts, richer Locus status/query primitives, Loom core block-edge graph primitives, Docs/Tiptap editor embodiment, and direct job-consent / MEX-Flight Recorder interaction edges.
- [ADD v02.147] Ownership sweep: Appendix 12 now binds high-signal orphan primitives for capability snapshots/consent scope, AI job query-update envelopes, debug bundle manifest/validation/status contracts, storage artifact/index/retention contracts, and operator export/query responses; projection/export force multipliers are explicit in the interaction matrix instead of remaining implicit novel ideas.
- [ADD v02.148] Ownership-reduction deepening: Appendix 12 explicitly attaches Stage/media session-auth contracts, MultiModelSession, debug export inventory/request/target contracts, and shared RetentionReport ownership; Stage?Media Downloader, Model Session?AI Job, and Debug Bundle?Storage Portability edges are explicit.
- [ADD v02.150] Backend matrix expansion: Appendix 12 now makes workflow?debug/locus, job?debug, consent?debug, calendar?debug, stage?debug, and media?debug/storage correlations explicit; unresolved combo implications are materialized as stub work packets instead of remaining matrix-only ideas.
- [ADD v02.151] Backend export/evidence/portability sweep: Appendix 12 now makes Role Mailbox?Flight Recorder/storage, AI-Ready Data?Flight Recorder/storage, and Workflow Engine?storage portability correlations explicit; unresolved mailbox/AI-ready debug-bundle bridges and calendar-mailbox correlation are materialized as stub work packets.
- [ADD v02.152] Backend orchestration/projection/replay sweep: Appendix 12 now makes Spec Router?Flight Recorder/storage, Locus?Flight Recorder, and MCP/MEX?Debug Bundle correlations explicit; unresolved spec-router portability, Locus debug-bundle, and MCP/MEX evidence-export bridges are materialized as stub work packets.
- [ADD v02.153] Backend capability/diagnostic sweep: Appendix 12 now makes Capabilitiesâ†’Flight Recorder, Workflow/Spec Routerâ†’Capabilities, MCPâ†’Flight Recorder, and Diagnosticsâ†’Debug Bundle correlations explicit; unresolved cloud-consent evidence portability is materialized as a stub work packet.
- [ADD v02.154] Backend governance/export reciprocity sweep: Appendix 12 now backfills Governance Pack and Workspace Bundle as explicit backend export surfaces, adds Governance Packâ†’Workflow/Capabilities/Flight Recorder/Storage Portability edges plus Workspace Bundleâ†’Flight Recorder/Storage Portability edges, and reuses existing governance/bundle stubs for unresolved delivery work.

- [ADD v02.155] Calendar-centered backend sweep: Appendix 12 deepens Calendar as a sync-state, export-mode, capability, AI-job mutation, and scope-hint routing surface; adds Calendarâ†’Storage Portability / Capabilities & Consent / AI Job Model / Spec Router edges; and keeps mailbox/Locus/export bridges stub-backed until dedicated backend joins exist.

- [ADD v02.156] Knowledge/retrieval pillar backend sweep: Appendix 12 deepens Project Brain, Semantic Catalog, Context Packs, Loom, and AI-Ready Data as explicit backend retrieval contracts; adds Project Brainâ†’AI-Ready Data, Semantic Catalogâ†’Spec Router, Context Packsâ†’Storage Portability, and Loomâ†’Storage Portability edges; and materializes unresolved Loom portability delivery as a dedicated stub track.

- [ADD v02.157] Distillation/context/spec-router backend sweep: Appendix 12 deepens Skill Bank, Context Packs, ACE Runtime, Micro-Task Executor, and Spec Router as one backend learning substrate; adds ACE Runtimeâ†’Context Packs, Micro-Task Executorâ†’Skill Bank, Context Packsâ†’Flight Recorder, Skill Bankâ†’Storage Portability, and Spec Routerâ†’Context Packs edges; and materializes unresolved Context Pack recorder visibility as a dedicated stub track.

- [ADD v02.158] Stage/Studio/Media/ASR backend pillar sweep: Appendix 12 deepens ASR, Media Downloader, Stage, Studio, and Atelier/Lens as artifact-lineage-aware backend surfaces; adds ASRâ†’Flight Recorder, ASRâ†’Storage Portability, Media Downloaderâ†’ASR, and Stageâ†’Storage Portability edges; and materializes unresolved Stageâ†’ASR transcript lineage as a dedicated stub track.
- [ADD v02.159] Correlation/projection backend pillar sweep: Appendix 12 clarifies Dev Command Center as the control/projection umbrella, keeps Operator Consoles as the specialized evidence/debug cluster, adds Dev Command Centerâ†’Flight Recorder, Dev Command Centerâ†’Debug Bundle, and Role Mailboxâ†’Dev Command Center edges, and keeps weaker Locus/Role Mailbox/Calendar bundle bridges stub-backed.

- [ADD v02.160] Dev Command Center control-plane backend sweep: Appendix 12 deepens Dev Command Center as the governed control surface for workflow runs, artificial intelligence jobs, capability decisions, model sessions, and work packet or worktree bindings; adds Dev Command Center to Workflow Engine, Dev Command Center to Artificial Intelligence Job Model, Dev Command Center to Capabilities and Consent, and Dev Command Center to Model Session Orchestration edges; and reuses the existing Dev Command Center, workflow projection correlation, and consent audit projection backlog instead of creating duplicate control-plane stubs.

- [ADD v02.161] Dev Command Center evidence-and-replay backend sweep: Appendix 12 deepens Dev Command Center as the governed evidence and replay projection surface for Governance Pack export, Workspace Bundle export, diagnostics queries, and bounded workflow-linked evidence packaging; adds Dev Command Center to Governance Pack, Dev Command Center to Workspace Bundle, and Dev Command Center to Diagnostics Schema edges; and reuses the existing governance, workspace bundle, and diagnostics backlog instead of creating duplicate export or replay stub families.
- [ADD v02.162] Dev Command Center work-orchestration backend sweep: Appendix 12 deepens Dev Command Center as the governed projection surface for tracked Work Packet state, Task Board sync freshness, Micro-Task summaries, ready-query status, workflow-linked work packet activation, and parallel model session occupancy; adds Dev Command Center to Locus Work Tracking, Dev Command Center to Micro-Task Executor, and Model Session Orchestration to Locus Work Tracking edges; and reuses the existing Dev Command Center, Locus occupancy, multi-session orchestration, and micro-task summary backlog instead of creating duplicate orchestration stub families.
- [ADD v02.163] Dev Command Center planning-and-coordination backend sweep: Appendix 12 backfills Task Board and Work Packet System as first-class backend coordination features, deepens Dev Command Center/Locus/Workflow Engine/Model Session Orchestration/Micro-Task Executor ownership for task-board authority, work-packet binding, workflow-linked activation, and parallel-session planning, and adds Dev Command Center to Task Board, Dev Command Center to Work Packet System, Task Board to Locus Work Tracking, and Work Packet System to Workflow Engine edges while reusing existing Dev Command Center, Locus, and Spec Session Log backlog instead of creating duplicate planning-system stubs.
- [ADD v02.164] Dev Command Center resilience and governed repository-decision backend sweep: Appendix 12 deepens Dev Command Center as the governed recovery and decision surface for session checkpoints, heartbeat freshness, provider capability readiness, anti-pattern alerts, and repository-engine backend policy; adds Dev Command Center to Model Session Orchestration and Dev Command Center to Unified Tool Surface edges; and reuses the existing Dev Command Center, Provider Feature Coverage, Session Crash Recovery, Session Observability, Session Anti-Pattern Registry, Workflow Projection Correlation, and Git Engine Decision Gate backlog instead of creating duplicate recovery or repository-policy stubs.
- [ADD v02.165] Dev Command Center operating-surface backend sweep: Appendix 12 deepens Dev Command Center as the governed run-history, tool infrastructure, workspace-runtime, and promotion-gate surface; adds Dev Command Center to Workflow Engine and Dev Command Center to Unified Tool Surface edges for replay and tool-health projection; and reuses the existing Dev Command Center, Workflow Projection Correlation, Unified Tool Surface Contract, Workspace Safety Parallel Sessions, and Git Engine Decision Gate backlog instead of creating duplicate run-history or promotion-gate stub families.
- [ADD v02.166] Structured collaboration-substrate backend sweep: Appendix 12 deepens Locus Work Tracking, Micro-Task Executor, Role Mailbox, Task Board, Work Packet System, and Dev Command Center as one structured work-state substrate; adds Dev Command Center to Role Mailbox and Role Mailbox to Work Packet System edges for triage and handoff correlation; and reuses the existing Dev Command Center, Locus Work Tracking, Role Mailbox, and workflow backlog instead of creating duplicate structured-state stub families.
- [ADD v02.167] Canonical structured artifact backend sweep: Appendix 12 deepens Locus Work Tracking, Task Board, Work Packet System, Role Mailbox, and Dev Command Center around versioned JavaScript Object Notation file standards, compact summaries, project-agnostic profile envelopes, and projected board or queue layouts; adds Task Board to Work Packet System structured-board projection guidance; and materializes only genuinely new structured-artifact, mirror-sync, and typed-viewer implementation gaps as dedicated stubs.
- [ADD v02.168] Base structured schema and project-profile sweep: Appendix 12 deepens Locus Work Tracking, Micro-Task Executor, Task Board, Work Packet System, Role Mailbox, and Dev Command Center around shared base-envelope fields, compact summary contracts, mirror-state semantics, and profile-extension boundaries; and materializes only genuinely new schema-registry and project-profile-extension implementation gaps as dedicated stubs.
- [ADD v02.169] Canonical-to-mirror reconciliation sweep: Appendix 12 deepens Dev Command Center, Locus Work Tracking, Task Board, Work Packet System, Role Mailbox, and Micro-Task Executor around mirror authority mode, reconciliation action, advisory-edit posture, and normalization-safe Markdown regeneration; and reuses the existing Markdown mirror sync and typed viewer stubs instead of creating duplicate drift-governance tracks.
- [ADD v02.170] Dev Command Center layout-projection sweep: Appendix 12 deepens Dev Command Center, Task Board, Work Packet System, Role Mailbox, and Micro-Task Executor around typed view presets, lane definitions, governed action bindings, roadmap or inbox layouts, and local-small-model execution queues; adds execution-queue projection guidance; and materializes only genuinely new layout-projection registry work as a dedicated stub.
- [ADD v02.171] Workflow-state and governed-action sweep: Appendix 12 deepens Dev Command Center, Locus Work Tracking, Work Packet System, Micro-Task Executor, Task Board, and Role Mailbox around project-agnostic workflow-state families, queue-reason codes, and governed action descriptors; adds mailbox and queue-reason routing guidance; and materializes only genuinely new workflow-state registry work as a dedicated stub.
- [ADD v02.172] Workflow transition and executor sweep: Appendix 12 deepens Dev Command Center, Locus Work Tracking, Work Packet System, Micro-Task Executor, Task Board, Role Mailbox, and Workflow Engine around transition rules, automatic queue moves, approval-gated state changes, and executor eligibility; and materializes only genuinely new transition-automation registry work as a dedicated stub.
- [ADD v02.173] Role Mailbox message-contract sweep: Appendix 12 deepens Role Mailbox, Locus Work Tracking, Work Packet System, Micro-Task Executor, Task Board, and Dev Command Center around typed message families, thread lifecycle, delivery state, allowed-response envelopes, mailbox-local versus governed actions, and Micro-Task collaboration loops; and materializes only genuinely new mailbox-contract work as a dedicated stub.
- [ADD v02.174] Role Mailbox and Micro-Task loop-control sweep: Appendix 12 deepens Role Mailbox, Micro-Task Executor, Locus Work Tracking, Work Packet System, Task Board, and Dev Command Center around bounded loop checkpoints, structured verifier outcomes, retry-budget posture, escalation targets, dead-letter loop handling, and completion-report transcription; deepens Role Mailbox to Micro-Task Executor and Role Mailbox to Work Packet System guidance; and materializes only genuinely new mailbox-loop-control work as a dedicated stub.
- [ADD v02.176] Role Mailbox executor-routing and claim-lease sweep: Appendix 12 deepens Role Mailbox, Dev Command Center, Locus Work Tracking, Micro-Task Executor, Task Board, and Work Packet System around executor-kind routing, exclusive versus shared claim modes, claimant visibility, lease age and expiry, response-authority scope, takeover legality, and human-only override boundaries; deepens mailbox claimant and actor-eligibility guidance; and materializes only genuinely new mailbox claim-lease work as a dedicated stub.
- [ADD v02.177] Role Mailbox handoff-bundle and announce-back sweep: Appendix 12 deepens Role Mailbox, Work Packet System, Locus Work Tracking, Micro-Task Executor, Task Board, and Dev Command Center around structured handoff bundles, note-transcription provenance, announce-back kinds, compact handoff summaries, and replay-safe operator views; deepens mailbox-to-packet and mailbox-to-Dev-Command-Center handoff guidance; and materializes only genuinely new mailbox handoff and transcription work as a dedicated stub.
- [ADD v02.178] RAG mode and no-RAG sweep: Appendix 12 deepens Project Brain, AI-Ready Data, ACE Runtime, Loom, Prompt-to-Spec Router, Work Packet System, and Micro-Task Executor around retrieval-mode selection, non-hybrid reasons, Loom-driven graph bias, authoritative direct loads, and bounded local-model context assembly; adds Loom -> Project Brain, Loom -> Prompt-to-Spec Router, Prompt-to-Spec Router -> Work Packet System, and Work Packet System -> Micro-Task Executor edges; and materializes only genuinely new retrieval-mode policy work as a dedicated stub.

- [ADD v02.181] Software-delivery governance overlay boundary sweep: Phase 1 MUST keep repository `/.GOV/**` artifacts as imported overlay source material and evidence while live software-delivery authority moves through product-owned runtime records and workflow-backed governed actions.
- [ADD v02.181] Software-delivery runtime-truth sweep: Phase 1 MUST expose software-delivery work through stable-id-linked runtime records, linked governed actions, and workflow-backed state rather than packet text, mailbox order, or Markdown mirrors acting as operational truth.
- [ADD v02.181] Validator-gate and closeout sweep: Phase 1 MUST converge validator posture into runtime-visible gate summaries and evidence-linked gate executions, and MUST derive closeout posture from canonical runtime and gate state rather than packet surgery.
- [ADD v02.181] Projection-surface sweep: Phase 1 MUST keep Dev Command Center, Task Board, and Role Mailbox as projection or control surfaces over the same runtime truth, with no planning lane, inbox thread, or readable mirror becoming authority by chronology alone.
- [ADD v02.181] Overlay coordination-record sweep: Phase 1 MUST model claim/lease posture and queued steering or follow-up posture explicitly where software-delivery control requires ownership, takeover, renewal, escalation, or deferred steering semantics.
- [ADD v02.181] Overlay lifecycle and recovery sweep: Phase 1 MUST expose lifecycle checkpoints and workflow-backed start/steer/cancel/close/recover semantics so crash recovery, restart-safe steering, and partial-failure handling remain replayable and explainable.

- Calendar range selection returns the same ActivitySpan/SessionSpan set as the equivalent Flight Recorder interval-overlap query (filters + attribution mode recorded).


- For every AI action in the UI, a corresponding AI job and workflow run exists and can be inspected.  
- Flight Recorder shows a coherent timeline for at least the core loop.  
- Metrics and logs are sufficient to explain failures in the core loop without reading the entire codebase.
- Operator Consoles v1 exists (Timeline + Jobs + Problems + Evidence) and every entry deep-links to the underlying trace/events.  
- Debug Bundle export is redacted-by-default, deterministic for the same selection, and passes the validator pack in CI.  
  
- Bootloader/Diary checks for logging and non-silent edits pass in CI.
- Data layer invariants enforced: Raw/Derived/Display separation respected; layer_scope/apply_scoped/preview_only/access_mode persisted; per-op provenance visible in Flight Recorder.  
- At least one end-to-end MCP-backed job (stub server is fine) is visible in Job History and Flight Recorder, with Gate decisions and capability metadata attached.
- [ADD v02.136] Tool Registry + Tool Gate: at least 10 tools are registered with stable names/versions and side_effect labels; local and MCP tool discovery expose the same HTC schema; every tool call emits FR-EVT-007 (ToolCallEvent) with redaction; Tool Contract conformance tests (Â§6.0.2.9) pass in CI.
- [ADD v02.138] Front End Memory System (FEMS): `SESSION_SCOPED` and `WORKSPACE_SCOPED` policies inject a bounded `MemoryPack` (â‰¤500 tokens) visible in DCC; procedural memory proposals require explicit approval; `FR-EVT-MEM-*` are emitted; replay reproduces the same `memory_pack_hash`; FEMS-EVAL-001 passes.
- [ADD v02.178] Retrieval mode discipline: `QueryPlan` and `RetrievalTrace` record `retrieval_mode` and `non_hybrid_reason`; known Work Packet or Micro-Task ids and exact Loom or artifact refs bypass hybrid retrieval by default; Project Brain and Prompt-to-Spec use hybrid retrieval only for discovery or synthesis; and local-small-model Micro-Task loops consume bounded direct loads or compacted snippets rather than broad Project Brain retrieval by default.
- Migrations validated: forward/backward fixture tests pass (up + down), replay-safety test passes (replay all up migrations), and migration version surfaces in a health check.  
- Workflow/Job completeness: mandatory fields (job_kind/profile_id/layer_scope/EntityRef) recorded; idempotency keys honored; retries capped; crash/restart yields resumed or failed runs with clear status.
- Capability model is default-deny across AI/mechanical/terminal/Monaco; approvals cached with TTL; allow/deny decisions logged in Flight Recorder.
- Retention/redaction defaults enforced: FR/log retention windows applied; redacted output retention window honored; env/secret scrubbing verified.
- [ADD v02.101] Spec Router produces SpecIntent + SpecRouterDecision artifacts with pinned capability_registry_version; Task Board and Work Packet entries are created/updated and visible in a Spec Session Log view.
- [ADD v02.128] Spec Creation System v2.2.1 is usable in-app (command palette/CLI): `/spec new|extend|refine|check` and `/task` routes are command-driven, emit the correct artifacts, and update `SPEC_INDEX.yaml` deterministically.
- [ADD v02.128] `/spec check` emits a rubric report that includes second-model + red-team passes; hard failures block GOV_STRICT activation and are visible in Problems/Evidence + Spec Session Log.
- [ADD v02.128] Universal ID + requirement grammar validation runs on spec artifacts; violations are explained and block activation in GOV_STRICT/GOV_STANDARD.
- [ADD v02.101] Git workflows require safety commit before execution; non-git workflows skip the safety commit step by policy.
- [ADD v02.101] Atelier Lens claim/glance runs on all ingested artifacts by default; disable only via LAW override.
- **[ADD v02.63] Model profile clarity:** Runtime integration documents and ships a concrete ModelProfile/Routing/SafetyProfile schema for MVP models (id, role, safety policy, routing notes) and evidence of usage in jobs.
- Terminal LAW (minimal slice): run_command defaults to policy mode with timeout (~180s), kill_grace, and max_output_bytes (~1â€“2MB); approvals UI present; sessions bound to workspace; executions logged in Flight Recorder.
- CI gates: lint/format/test and health script enforced in CI; fail if logging/FR hooks are missing.
- Sheets MVP present (v02.70). Implementation details moved to **Â§2.2.1.13**.
  - HyperFormula formulas, basic grid operations, and import/export fixture pass.
- Safety/ops engines exercised: Guard/Container/Quota enforced with FR evidence; Profiler/Monitor metrics visible per job; Repo/Formatter/Deploy paths logged via capability gates; Clipboard/Notifier actions bound to consent/capability.
- Atelier foundation present: create/edit `AtelierProductionPlan`, run `AtelierCompiler` to emit deterministic prompt/design/comfy_recipe exports with provenance; internal storage remains raw.



- [ADD v02.36] Debug Bundle for a recorded job includes ContextPlan + ContextSnapshots + validator outcomes; validator pack runs in CI.
- [ADD v02.36] Terminal LAW tests exist: denied commands are denied with logged gate decision; allowed commands are fully traced with scrubbed env.
- [ADD v02.38] Design Pack fonts load offline and include required licensing artifacts (per-font license files + THIRD_PARTY_NOTICES).  
- [ADD v02.38] Font Registry import/remove updates manifest deterministically and is visible in UI (list families).  
- [ADD v02.38] Canvas text uses bundled fonts without fallback flash; Canvas export preserves selected font.


- [ADD v02.42] Any retrieval performed in Phase 1 is snippet-only (`fetch_depth = snippet`), budgeted, and logged with EvidenceSnippet rationale; no full-page dumps enter the model context.

- [ADD v02.44] Build/release fails if any shipped OSS dependency is absent from the OSS Component Register or violates isolation rules.
- [ADD v02.44] Supply-chain gates run end-to-end and store their reports/SBOM/license outputs as artifacts, visible in Jobs + Flight Recorder.
- [ADD v02.44] Export deliverable PDF is available as a Job and produces a stored artifact via Typst + qpdf.
- [ADD v02.49] At least two Phase-1 mechanical jobs produce artifacts that (a) have manifests, (b) hash with SHA-256, (c) are discoverable via a minimal artifact viewer/list, and (d) survive restart.
- [ADD v02.49] Pin/unpin + TTL + GC can be demonstrated end-to-end; GC does not delete pinned; retention report is stored as an artifact.
- [ADD v02.49] Any â€œexport to file pathâ€ uses atomic materialize and logs policy + hashes (no direct writes).

- [ADD v02.52] For every retrieval-backed call: QueryPlan + RetrievalTrace exist, are hashed, and are reachable from Job History/Operator Consoles; evidence items are bounded and carry SourceRefs or ArtifactHandles.
- [ADD v02.52] CI runs T-ACE-RAG-001 (normalization determinism) and T-ACE-RAG-002 (strict ranking determinism) on a fixed fixture corpus; failures surface as Problems with Debug Bundle linkage.
- [ADD v02.52] Debug Bundle meets required structure and emits its export event (per existing Master Spec).
- [ADD v02.52] Workspace Bundle contains required tree and manifest; produces stable hashes when rerun with identical inputs/profile.
- [ADD v02.52] Policy context is captured/visible for export actions.
- [ADD v02.52] Attempt to include `exportable=false` artifacts without explicit policy is denied, logged, and surfaced.


- [ADD v02.67] Atelier Lens v0.1 runs end-to-end on a mixed-domain fixture set; Role Claims + Role Glances are visible, and top-k role extraction produces evidence-linked `RoleDescriptorBundle`s.
- [ADD v02.67] `ATELIER_STATE_MERGE` produces deterministic `SceneState.resolved` hashes under pinned inputs and emits `ConflictSet` whenever conflicts exist; `ATELIER-LENS-VAL-007/008` pass on golden fixtures.
- [ADD v02.67] `ConceptRecipe` is generated from Artistic Vectors, persisted with pins, and replayable; `ATELIER-LENS-VAL-009` passes.
- [ADD v02.67] `AtelierProductionGraph` is solvable and produces a stable `solve_plan`; `ATELIER-LENS-VAL-010/011` pass.
- [ADD v02.67] At least one Finishing role emits a typed deliverable bundle (e.g., grade targets/LUT spec, edit beat map, VFX shot list) with evidence refs and Flight Recorder provenance.


- [ADD v02.68] At least 3 mechanical engines pass Conformance Harness v0; Engine PlannedOperation/EngineResult envelopes validate, required gates run, and denials are visible in Problems/Flight Recorder.
- [ADD v02.68] Artifact-first engine I/O is enforced: payloads >32KB are artifact refs, outputs are artifacts with SHA-256 + provenance manifests, and D0/D1 runs include evidence artifacts when applicable.
- [ADD v02.79] Can import a folder of images, render a grid, open a single image, and export a derivative via job history with traceable inputs/outputs.


- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - Two WPs can run concurrently iff IN_SCOPE_PATHS are disjoint; otherwise one blocks with CX-MM-002 and a lock conflict report.  
  - System runs in DOCS_ONLY with zero models, and in AI_ENABLED only with >=1 READY model.  
  - Every role output includes RoleExecutionIdentity metadata and HSK_STATUS updates on phase transitions.  
  - SwapRequest escalation can replace a failing smaller model with a larger model, preserving WP/MT state.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - Operator can open DCC, select a WP, open its linked worktree, view diff, approve a needed capability, and run a governed commit without leaving Handshake.  
  - Every stateful DCC action emits Flight Recorder events and is traceable to `wp_id`/`mt_id`/`session_id`/`wsid`.  
  - Denied approvals block the job deterministically with an explicit failure code and no partial side-effects.  
  - `.handshake/workspace.json` + `devcc.db` can be deleted and rebuilt from repo state + Locus without corrupting canonical governance artifacts.
- [ADD v02.164] Dev Command Center resilience and repository decision posture: orphaned or blocked sessions surface checkpoint age, heartbeat freshness, pending tool-call count, and governed resume or cancel actions; provider readiness surfaces by stable session and workflow identifiers; and the version-control panel exposes the declared repository backend, required status checks, merge-queue compatibility, and explicit no-silent-fallback posture before protected-branch actions proceed.
- [ADD v02.165] Dev Command Center operating surfaces: run history exposes workflow, node, tool, and checkpoint chronology with replay entrypoints; tool infrastructure registry exposes transport, health, permission scope, route policy, and last failure; workspace runtime exposes isolation posture, startup readiness, and external-workspace state; and promotion gate snapshots expose review resolution, required checks, merge-queue posture, and last verification timestamps before protected-branch actions proceed.
- [ADD v02.166] Structured collaboration substrate: at least one Work Packet, one Micro-Task, one Task Board view, and one Role Mailbox thread can be inspected through Dev Command Center typed-field viewers; local small models consume bounded structured fields without parsing raw Markdown contracts; and append-only notes plus Markdown mirrors remain synchronized with the authoritative structured records.
- [ADD v02.167] Canonical structured artifacts: at least one Work Packet, one Micro-Task, one Task Board projection, and one Role Mailbox thread exist as versioned JavaScript Object Notation or JavaScript Object Notation Lines artifacts with compact summaries and project-agnostic base envelopes; Dev Command Center can render at least two different board or queue layouts from the same structured records without changing authority; and Markdown mirrors are detectable as synchronized, stale, or advisory.
- [ADD v02.168] Base schema and project-profile contracts: at least one Work Packet, one Micro-Task, one Task Board projection, and one Role Mailbox export family member declare the same base structured-collaboration envelope fields, expose compact summary records, and isolate project-specific fields inside explicit profile extensions; Dev Command Center can distinguish base-envelope versus profile-extension fields without raw-file inspection; and mirror-state handling remains deterministic across canonical detail, summary, and Markdown mirror views.
- [ADD v02.169] Canonical-to-mirror drift governance: at least one Work Packet, one Micro-Task, one Task Board projection, and one Role Mailbox export family member declare mirror authority mode and reconciliation action explicitly; Dev Command Center can show why a Markdown mirror is synchronized, stale, advisory, or normalization-required; and regenerating readable mirrors does not silently overwrite append-only note sidecars or operator-authored advisory content.
- [ADD v02.170] Typed operating layouts: Dev Command Center can switch between board, queue, list, roadmap, inbox-triage, and execution-queue presets over the same canonical records; each drag, quick action, or bulk action previews the governed target fields or workflow actions before mutation; and local-small-model readiness queues remain derivable from compact summaries plus mailbox-response state.
- [ADD v02.171] Project-agnostic workflow-state contracts: at least one Work Packet, one Micro-Task, one Task Board projection, and one Role Mailbox-linked queue row declare `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` explicitly; Dev Command Center can show why a record is in intake, ready, waiting, review, approval, validation, blocked, done, canceled, or archived state; and local-small-model routing remains derivable from base families plus reason codes even when project-profile labels differ.
- [ADD v02.172] Workflow transition and executor eligibility contracts: at least one Work Packet, one Micro-Task, one Task Board projection, and one Role Mailbox-linked wait expose the transition rule, automatic queue-action rule, and executor eligibility policy that explain why a record may start, wait, escalate, retry, request review, request approval, validate, complete, or remain blocked; and Dev Command Center can preview whether an action is automatic, approval-gated, or actor-ineligible before state changes occur.
- [ADD v02.173] Role Mailbox message and thread contracts: at least one Work Packet handoff thread, one Micro-Task request or feedback thread, and one escalation or announce-back thread expose thread lifecycle state, message delivery state, allowed responses, due or snooze posture, linked stable identifiers, and the authority boundary between mailbox-local actions and governed record mutations; and Dev Command Center can preview that distinction before a reply, acknowledgement, escalation, or transcription action executes.
- [ADD v02.174] Role Mailbox and Micro-Task loop control: at least one Micro-Task request thread, one feedback or retry loop, one verification-needed thread, and one escalation or completion-report thread expose remaining retry budget, structured verifier outcome, escalation target, last loop checkpoint, and linked stable identifiers; Dev Command Center can preview whether the next action is mailbox-local, governed retry, governed escalate, governed complete, or transcription-only; and linked Work Packet and Task Board views can explain why a task is waiting, retrying, escalated, or complete without replaying the full thread.
- [ADD v02.175] Role Mailbox triage and remediation controls: at least one snoozed thread, one due-soon or expired thread, and one dead-letter remediation thread expose triage queue state, reminder schedule, queue age, dead-letter disposition, and linked stable identifiers; Dev Command Center can preview whether reminder, unsnooze, retry-delivery, reroute, transcription, or archive controls are mailbox-local, automation-triggering, or governed; and linked Task Board, Work Packet, and Locus views can explain mailbox-derived waiting pressure without using thread order as authority.
- [ADD v02.176] Role Mailbox executor routing and claim-lease control: at least one exclusively leased thread, one shared-observer or broadcast thread, and one takeover or lease-expiry case expose executor kind, claimant identity, claim mode, lease age, lease expiry, response-authority scope, and linked stable identifiers; Dev Command Center can preview whether claim, release, renew, reroute, takeover, or reply actions are mailbox-local, automation-triggering, or governed; and linked Locus, Micro-Task, Work Packet, and Task Board views can explain who may act next without relying on assignment comments or transcript order.
- [ADD v02.177] Role Mailbox handoff bundle and announce-back provenance: at least one delegate or handoff thread, one completion-report or announce-back thread, and one scope-change or escalation handback expose remaining work, unresolved blockers, recommended next actor, evidence refs, provenance kind, and transcription status by stable identifiers; Dev Command Center can distinguish advisory announce-back from transcription-confirmed completion; and linked Work Packet, Locus, Micro-Task, and Task Board views can resume from compact handoff state without replaying the full thread.
- [ADD v02.181] Software-delivery governance overlay boundary: at least one workflow-backed software-delivery flow preserves repository `/.GOV/**` artifacts as imported overlay source material or evidence, and Governance Pack import/export preserves those artifacts without bypassing workflow-backed runtime law.
- [ADD v02.181] Software-delivery runtime truth: at least one workflow-backed software-delivery work item exposes product-owned runtime state and linked governed actions by stable identifiers instead of relying on packet prose, mailbox order, or Markdown mirrors as the operational authority surface.
- [ADD v02.181] Validator-gate and closeout posture: at least one workflow-backed software-delivery work item exposes validator-gate summaries, evidence-linked gate posture, and derived closeout posture by stable identifiers without requiring packet surgery to explain why the item may proceed or close.
- [ADD v02.181] Projection-surface discipline: Dev Command Center, Task Board, and Role Mailbox projections for at least one software-delivery work item explain the same underlying state without turning repo `/.GOV/**`, Markdown mirrors, or mailbox chronology into authority.
- [ADD v02.181] Overlay coordination records: at least one software-delivery work item exposes overlay claim/lease state and queued steering or follow-up state by stable identifiers so actor ownership, takeover legality, and deferred steering are visible without transcript reconstruction.
- [ADD v02.181] Overlay lifecycle and recovery posture: at least one software-delivery work item exposes checkpoint-backed recovery posture plus workflow-backed start/steer/cancel/close/recover semantics by stable identifiers so restart-safe replay and control decisions remain inspectable.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - If `approximate.allowed=false` or waiver missing, approximate execution cannot occur; system downgrades to exact and logs the downgrade.  
  - For calls with `settings.exec_policy`, Flight Recorder contains an `llm_exec_policy` event capturing requested vs effective policy.  
  - If approximate execution occurs, the event includes `waiver_ref` and a trace artifact reference (or explicit `trace_artifact_ref=null` with reason).

**Explicitly OUT of scope**
- [ADD v02.130] Loom: AI auto-tagging/auto-caption, semantic/hybrid search (Tierâ€‘3), multi-user Loom collaboration/sync, and Postgres-backed Loom query engines are Phase 2+ / Phase 4.
- [ADD v02.131] Stage: browser-extension ecosystem, arbitrary third-party Stage Apps/plugin marketplace, bulk crawling/mirroring, Stage Studio authoring (Spline-class editor), and advanced 3D editing/collaboration are Phase 3+ / Phase 4.


- [ADD v02.47] Charts/dashboards, decks, and any PPTX/PDF export pipelines (including in-app presentation surfaces).

- External calendar sync (CalDAV) and any external write-back.
- Multiple LLM runtimes and sophisticated model routing.  
- Sheets engine beyond a minimal stub (tables can be represented, but no full formula engine yet).  
- Docling ingestion, ASR pipeline, connectors, plugin system.  
- Multi-user sync and CRDT.
- High-performance LLM runtimes (e.g., `vLLM`, `TGI`) or cloud-scale routing beyond the single local runtime.  
- Advanced image generation stacks (`SDXL`, `ComfyUI`) and related workflows.  
- Full spreadsheet functionality beyond a minimal grid display (HyperFormula formulas stay stubbed in Phase 1).  
- Observability dashboards (e.g., `Grafana`, `Jaeger`) beyond the built-in MVP diagnostics surfaces.  

- Paid/proprietary font distribution and cloud font providers (e.g., Adobe Fonts).  
- Advanced OpenType feature UI (liga/ssXX) and variable-font axis controls beyond basic weight/italic.  
- Font editing/subsetting workflows.

- [ADD v02.49] Cross-device artifact sync/dedup, multi-user artifact lineage, and advanced GC heuristics beyond TTL+pinning.

- [ADD v02.52] ContextPacks builder job, pack freshness guard, index drift guard, and hash-key caching effectiveness (candidate/rerank/span caches) â€” Phase 2.
- [ADD v02.52] Format round-tripping (DOCX/PPTX/XLSX writers).
- [ADD v02.52] Cloud bundle sharing/upload.
- [ADD v02.52] Export workflows that mutate Raw/Derived stores.

- [ADD v02.79] Photo Studio advanced features: RAW decode, lens corrections, masks, layer compositor, HDR/pano/focus merges, AI vision, ComfyUI, vector tools.


- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - Any GPU-sharded inference of a single model.  
  - Any requirement for multiple GPUs.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - GitHub PR/comment sync.  
  - Multi-user workspace sync / shared approvals.  
  - Full external conversation ingestion (beyond adapter skeleton + at least one external pilot, if any).  
  - Any UI commitment to Sidecar keybindings/TUI parity.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - Implementing true layer-wise inference in local runtimes (LayerSkip/early-exit/etc).  
  - Multi-device/sharded inference.

**Mechanical Track (Phase 1)**
- [ADD v02.130] Loom media mechanicals: deterministic content hashing + dedup, background thumbnail/proxy generation jobs, and periodic recomputation of LoomBlock metrics (backlink/mention/tag counts).
- [ADD v02.131] Stage capture mechanicals: wire minimal `Archivist` (web capture + evidence bundling) and `Guide` (live verify) as Tool Bus engines; add Stage capture/import job profiles (`stage.capture_webpage.v1`, `stage.clip_selection.v1`, `stage.import_pdf.v1`) and 3D validators (`stage.3d.validate_gltf.v1`) producing artifact bundles with manifests, hashes, and replayable provenance.
- [ADD v02.136] Populate **Tool Registry** (HTC v1.0) for all Phase 1 engines/tools (`Context`, `Version`, `Sandbox`, `Publisher`, `Formatter`, `Repo`, `Deploy`, `Guard`, `Container`, `Quota`, `Profiler`, `Monitor`, `Clipboard`, `Notifier`) and enforce calls through Tool Gate (no-bypass).

- Deliver low-risk local engines: `Context` (rg), `Version` (Jujutsu/Gitoxide), `Sandbox` (safe code exec), `Publisher` (deterministic Markdown/Doc to PDF), `Formatter` (lint/format enforcement), `Repo` (git/libgit actions), and `Deploy` (minimal devops automation).
- Add safety/observability primitives: `Guard` (secret/safety scan), `Container` (isolated exec), `Quota` (resource limits), `Profiler`/`Monitor` (system metrics/alerts), `Clipboard` (controlled ephemeral context), and `Notifier` (desktop notifications).
- MVP engine implementations MUST be demonstrable end-to-end: `Context` using `rg` for text search, `Version` using git/libgit for repo state, `Formatter` using `ruff` and `prettier`, and `Guard` using `trufflehog` for secret scanning, all running through the mechanical runner abstraction and emitting Flight Recorder provenance.
- All mechanical jobs MUST run via the Workflow Engine with capability checks; log command, params, exit code, stdout/stderr, artifact hash, and store DerivedContent + sidecar provenance.
- Acceptance: at least two mechanical job profiles visible in Job History with capability enforcement tests and reproducible commands; safety/resource gates (Guard/Container/Quota/Profiler/Monitor) exercised and logged; Clipboard/Notifier actions bound to capability/consent.

- [ADD v02.115] **AI-Ready Data Architecture (Â§2.3.14) - Phase 1:**
  - Implement Bronze/Silver/Gold storage layers mapped to `workspace/raw/`, `workspace/derived/`, `workspace/indexes/`
  - Implement content-aware chunking for code (AST-aware, 100-500 tokens) and documents (header-recursive, 256-512 tokens)
  - Implement embedding pipeline with model versioning (`text-embedding-3-small` default, `bge-small-en-v1.5` local fallback)
  - Implement hybrid search (vector HNSW + keyword BM25) with configurable weights
  - Implement ingestion validation gates (token count, coherence checks, boundary validation)
  - **[REMEDIATION]** Wire FR-EVT-DATA-001 through FR-EVT-DATA-015 events to existing Flight Recorder (new event schemas for bronze/silver/embedding/retrieval/quality)
  - Implement quality SLOs and alerts (MRR â‰¥ 0.6, Recall@10 â‰¥ 0.8, p95 retrieval â‰¤ 500ms)
  - Acceptance: hybrid search returns results from Monaco, Terminal, and basic docs; retrieval traces visible in Operator Consoles; FR-EVT-DATA events appear in Flight Recorder


- [ADD v02.36] At least one mechanical job attempt is *denied* by capability/consent and the denial is visible in Problems + Flight Recorder (no side effects).
- [ADD v02.38] Font Registry mechanical job(s): `fonts_bootstrap_pack`, `fonts_rebuild_manifest`, `fonts_import`, `fonts_remove` (capability-gated; provenance recorded in Flight Recorder).  
- [ADD v02.38] Font pack manifests and per-font license metadata stored as DerivedContent with hashes; UI does not crawl the filesystem for font discovery.


- [ADD v02.44] Supply-chain gate mechanical Jobs (CI-gated): `secret_scan` (gitleaks), `vuln_scan` (osv-scanner), `sbom_generate` (syft), `license_scan` (scancode), each emitting artifacts + provenance.
- [ADD v02.44] OSS Register audit mechanical Job: `oss_register_audit` verifies (1) every integrated component has a Register entry and (2) integration mode matches license policy (GPL/AGPL isolation).
- [ADD v02.44] Git engine integration decision gate: record and enforce a single MVP path (`git` CLI `product_managed_process` vs `libgit2` vs `go-git`); default to `git` CLI `product_managed_process` until a decision is logged.
- [ADD v02.49] Artifact store bootstrap: create workspace `.handshake/artifacts/L{1,2,3}/<artifact_id>/â€¦` and write artifact manifests for every job output; hashes are SHA-256 everywhere (no SHA1 drift).
- [ADD v02.49] Materialize API: ALL â€œsave/export to pathâ€ writes go through one atomic materialize function (tmp + rename), capability-gated and Flight Recorder logged; no direct UI bypass.
- [ADD v02.49] Retention/pinning MVP: implement pin/unpin + TTL + a deterministic GC job/command; GC never deletes pinned artifacts; emit a retention report artifact for visibility.
- [ADD v02.49] Bundle canonical hashing: implement canonical bundle hashing (zip normalization) and use it for any bundle-style artifact (debug bundles, packaged deliverables, multi-file exports).

- [ADD v02.52] Retrieval trace bundle exporter: a mechanical job that takes `trace_id` (and referenced artifacts) and emits a redacted-by-default Debug Bundle artifact for retrieval issues (QueryPlan/Trace + budgets + cache keys + selected spans).
- [ADD v02.52] Deterministic bounded-read span extractor (mechanical helper) used by retrieval escalation paths; emits span selection provenance and truncation flags.
- [ADD v02.52] Job profiles (capability-gated; logged; hashed): `debug_bundle_export_v0`, `workspace_bundle_export_v0`.


- [ADD v02.67] Add mechanical job profiles + runner integration for Atelier Lens: `atelier_claim_v0`, `atelier_role_extract_v0`, `atelier_role_compose_v0`, `atelier_state_merge_v0`, `atelier_graph_solve_v0`, `atelier_concept_recipe_v0`; all capability-gated and Flight Recorder logged with pins/hashes.
- [ADD v02.67] Add CI fixtures + validators for Lens runs (VAL-007..011) and ensure denials/FAILs surface in Problems (no silent failures).


- [ADD v02.68] Mechanical Extension v1.2 enforcement: engine jobs use `poe-*` envelopes; required gates (`G-SCHEMA/G-CAP/G-INTEGRITY/G-BUDGET/G-PROVENANCE/G-DET`) run and are logged; engine registry is authoritative; conformance must pass before an engine is enabled for general use.
- [ADD v02.79] Add Darkroom engine stubs behind Mechanical Extension v1.2 (non-functional OK) so Photo Studio UI cannot mutate state outside the job/gate pipeline.


- [ADD v02.115] **Micro-Task Executor core loop (Â§2.6.6.8):** Implement MT Loop Controller with auto-generation of MT definitions from Work Packet scope, fresh-context-per-iteration execution, completion signal parsing with anti-gaming rules, and bounded iteration limits.
- [ADD v02.115] **MT validation engine wiring:** Wire validation commands through Mechanical Tool Bus (Â§6.3, Â§11.8) with PlannedOperation envelope, capability checks, and FR-EVT-MT-012 event emission.
- [ADD v02.115] **MT state persistence:** Implement ProgressArtifact and RunLedger schemas with atomic writes, crash recovery (Â§2.6.6.8.6.3), and FR-EVT-WF-RECOVERY integration.
- [ADD v02.115] **MT escalation chain:** Implement default 6-level escalation (7Bâ†’7B-altâ†’13Bâ†’13B-altâ†’32Bâ†’HARD_GATE) with LoRA selection by task_tags (auto_by_task_tags strategy).
- [ADD v02.115] Acceptance: MT Executor job profile (`micro_task_executor_v1`) visible in Job History; at least one Work Packet executes end-to-end with auto-generated MTs; escalation triggers FR-EVT-MT-005; hard gate pauses execution.

- [ADD v02.116] **Locus Work Tracking System (Â§2.3.15) - Phase 1:**
  - Implement SQLite backend with work_packets, micro_tasks, mt_iterations, dependencies tables (Â§2.3.15.5)
  - Implement core operations: locus_create_wp, locus_update_wp, locus_gate_wp, locus_close_wp, locus_register_mts, locus_start_mt, locus_record_iteration, locus_complete_mt (Â§2.3.15.3)
  - Implement dependency operations: locus_add_dependency, locus_remove_dependency with cycle detection (Â§2.3.15.7)
  - Implement basic query operations: locus_query_ready (dependency-aware), locus_get_wp_status, locus_get_mt_progress (Â§2.3.15.7)
  - Wire Spec Router integration: auto-invoke locus_create_wp when routing prompts, link to task_packet_path (Â§2.3.15.4)
  - Wire MT Executor integration: auto-invoke locus_start_mt, locus_record_iteration (every iteration), locus_complete_mt (Â§2.3.15.4)
  - Implement Task Board bidirectional sync: locus_sync_task_board reads/writes .handshake/gov/TASK_BOARD.md, auto-sync on WP state change (Â§2.3.15.4)
  - [ADD v02.116] **Task Board hygiene:** Task Board items tagged `v02.116` MUST be revised/updated (status, scope, owner, links). Ensure 1:1 mapping between Task Board entries and Locus `wp_id`s; remove stale/duplicate entries; re-run `locus_sync_task_board` to normalize.
  - Implement Bronze/Silver/Gold storage: WPBronze snapshots, WPSilver chunks with embeddings (text-embedding-3-small), basic keyword search (Â§2.3.15.5)
  - Wire Flight Recorder events: FR-EVT-WP-001..005, FR-EVT-MT-001..006, FR-EVT-DEP-001..002, FR-EVT-TB-001..003, FR-EVT-QUERY-001 (Â§2.3.15.6)
  - [ADD v02.116] Capability Registry update: ensure `locus.read`, `locus.write`, `locus.gate`, `locus.delete`, `locus.sync` are present in `CapabilityRegistry` SSoT; regenerate `assets/capability_registry.json`; add `HSK-4001: UnknownCapability` tests for all Locus operations.
  - [ADD v02.116] Flight Recorder schema validator update: register and validate Locus event families (FR-EVT-WP-001..005, FR-EVT-MT-001..006, FR-EVT-DEP-001..002, FR-EVT-TB-001..003, FR-EVT-SYNC-001..003, FR-EVT-QUERY-001); unknown event_type MUST fail fast in Diagnostics.
  - [ADD v02.116] Spec Router WorkPacketBinding enforcement: `work_packet_id` MUST be a valid existing Locus `wp_id`; invalid/missing MUST fail with Diagnostics and MUST NOT produce side effects (no writes, no external calls).
  - Acceptance: Spec Router creates WPs visible in Locus; MT Executor iterations recorded; Task Board syncs within 5s; **Task Board entries tagged `v02.116` revised/updated with no drift after sync**; locus_query_ready returns dependency-aware ready work; FR events appear in Flight Recorder.


- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - Implement lock semantics + Task Board integration.  
  - Implement HSK_STATUS generator + FR event correlation.  
  - Implement MailboxKind taxonomy + persistence.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - Implement DCC backend services: project/workspace/session registry and `devcc.db` migrations.  
  - Implement Approval Inbox plumbing: collect pending approvals from capability gate; persist decisions; emit Flight Recorder events.  
  - Implement worktree management job wrapper (create/open/prune) with safe defaults and explicit rewrite consent for destructive ops.  
  - Implement VCS panel operations via `engine.version` operations (status/diff/commit) + artifact-first commit messages.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - Schema updates + validators (Work Profile v0.6; runtime settings allow `exec_policy`).  
  - Flight Recorder schema + retention/privacy enforcement for layerwise traces.

**Atelier Track (Phase 1)**
- Implement storage + versioning for `DerivedContent: AtelierProductionPlan` (prose-first brief headings always present; structured `PlanFields` footer).
- Add `ATELIER_PLAN` job profile and a minimal editor surface for creating/refining plans (no rendering required in Phase 1).
- Implement deterministic `AtelierCompiler` as a mechanical job to emit: `export:image_prompt_generic`, `export:graphic_design_brief`, and `export:comfy_recipe` (template_id + deterministic fallback).
- Wire Atelier validators (ATELIER-VAL-001..005) to plan save/compile; auto-fill defaults rather than prompting.
- Acceptance: plans can be authored, validated, compiled, and exported with provenance linking to input references; connector-specific filtering occurs only at Display/Export boundaries.

- [ADD v02.52] Any Atelier job step that consults workspace evidence MUST emit QueryPlan/Trace and obey RetrievalBudgets (no "hidden retrieval" inside compilers).
- [ADD v02.52] Workspace/Debug Bundles may include Atelier artifacts **only if policy allows**; filtering remains Display/Export-only.

- [ADD v02.101] Atelier Lens claim/glance is always-on for ingested artifacts and Spec Router inputs; disable only via LAW override.


- [ADD v02.67] Implement Lens surfaces: Role Claims Panel, Role Glances Grid, Role Bundle Viewer (with evidence highlights) and Deliverables Browser for `RoleDeliverableBundle`.
- [ADD v02.67] Wire `ATELIER_CLAIM` into ingestion surfaces so any imported image/text can be claimed by multiple roles (cross-domain by design).
- [ADD v02.67] Ship MVP role contracts (dual-contract) for: `dop.lighting`, `set.set_dressing`, `fashion.styling`, `graphic_design`, and one Finishing role (`finishing.color` or `finishing.editorial`).
- [ADD v02.67] Implement `ATELIER_CONCEPT_RECIPE` and pass `ConceptRecipe` into compilers/composers (Conceptual Mode becomes replayable; recipes are first-class artifacts).
- [ADD v02.67] Implement `ATELIER_STATE_MERGE` (SceneState + ConflictSet) and `ATELIER_GRAPH_SOLVE` (ProductionGraph + solve_plan) as mechanical jobs used by Atelier compilation flows.
- [ADD v02.79] Add Photo Studio worksurface shell: browser grid + viewer + metadata inspector (read-only metadata is acceptable in Phase 1).
- [ADD v02.123] Implement **Atelier Collaboration Panel (selection-scoped)** in the main text editor: a sidebar/panel that runs role passes against the current selection, shows per-role suggestions (multiple options preferred), supports checkmark selection, and applies changes **only** to the selected span (never touching non-selected text). All applied patches MUST be recorded with provenance (role_id, contract_id, model_id/tool_id, evidence refs, before/after spans).
- [ADD v02.123] Implement `LensExtractionTier` plumbing (Tier1 default) and surface it in Lens job traces; Tier1 MUST be the default for all ingestion/extraction unless explicitly escalated.
- [ADD v02.123] Enforce **lossless role catalog + append-only role registry** in runtime + validators: role_id stability, no reuse, and a blocking validator if a previously-declared role disappears from the registry snapshot.
- [ADD v02.123] Implement `ViewMode` UI + enforcement for Lens outputs: `SFW` mode MUST **hard-drop** any non-`sfw` results from retrieval/output while preserving evidence pointers internally; switching ViewMode MUST NOT mutate stored Raw/Derived artifacts.
- [ADD v02.123] Implement role-turn isolation (role reset + context window reset) as the default execution mode for role passes (claim/glance/extract) to keep small local models consistent and prevent cross-role contamination; record per-turn pins for deterministic replay.


- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - UI to show READY models, active Work Units, lock states, and compact lifecycle status.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - DCC kanban lanes for WP statuses (from Locus), with deep links to worktree, sessions, approvals, and Flight Recorder slices.  
  - UX for approvals: single compact list with previews and scoping (once/job/workspace).

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - Operator UX for per-role compute preset selection and waiver-bound â€œapproximateâ€ toggle.

**Distillation Track (Phase 1)**
- Define Skill Bank schema alignment and logging-only distillation job profiles (no training) using Workflow Engine.
- Capture teacher/student metadata, context refs, reward features, lineage fields, and data_signature/job_ids_json in Flight Recorder.
- Acceptance: distillation job schema is wired with capability gating; log entries include all mandatory fields; no model training or promotion yet.
- [ADD v02.157] Context Pack build/select/refresh and freshness decisions MUST emit bounded Flight Recorder evidence and stable pack hashes so distillation/replay/onboarding paths never depend on hidden cache state.
- [ADD v02.157] Pending distillation candidate queues, PromptEnvelope hashes, and tokenizer metadata MUST remain durable backend evidence when Context Packs or Spec Router artifacts shape teacher/student inputs.

- [ADD v02.52] Distillation jobs MUST record referenced QueryPlan/Trace ids (when retrieval-backed) so training/eval inputs are auditable and replayable.
- [ADD v02.52] Distillation log artifacts must respect `exportable` flags so bundles cannot leak local-only payloads.


- [ADD v02.79] Capture Photo Studio job traces as distillation-ready sequences (recordable workflows; no learning required in Phase 1).

- [ADD v02.115] **MT Executor escalation candidate capture (Â§2.6.6.8.8):** When escalation occurs with `enable_distillation=true`, generate DistillationCandidate artifacts with student_attempt (failed) and teacher_success (passed) snapshots, task_type_tags, and data_trust_score.
- [ADD v02.115] **MT escalation record schema:** Capture contributing_factors (syntax_error, logic_error, scope_violation, etc.) and remediation outcomes for LoRA training feedback.
- [ADD v02.115] Acceptance: FR-EVT-MT-015 (distillation_candidate) events emitted on escalation; DistillationCandidate artifacts stored with Skill Bank schema alignment; no LoRA training in Phase 1.

- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - None required.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - None required.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - Not required; keep as future.

- [ADD v02.137] Autonomous agent loops without operator oversight (full AutomationLevel.AUTONOMOUS is Phase 2+).
- [ADD v02.137] Cross-workspace session routing (sessions are workspace-scoped in Phase 1).
- [ADD v02.137] Real-time collaborative session sharing between multiple human operators.

### 7.6.4 Phase 2 â€” Ingestion & Shadow Workspace (Docling + RAG MVP)

**Goal**  
Make Handshake useful over **existing** files and unlock basic retrieval-augmented generation, reusing the existing AI Job, workflow, and observability stack. Maintain and extend debug surfaces for ingestion and retrieval.
- [ADD v02.123] Phase 2 Atelier/Lens focus: Docling-driven Lens enrichment, global facts index + role-lane retrieval, Tier2 auto-when-idle deep passes, and SymbolismProfile/lexicon growth surfaced in UI and operator consoles.
- [ADD v02.136] Begin **Design Studio shell/IA** alignment (see `Handshake_Design_Studio_Overhaul_v0.1.md`): treat worksurfaces as modules in one shell (rail + inspector + bottom drawers). This is **recontextualization** only; storage/layout/capability boundaries remain unchanged.



- [ADD v02.105] Phase coverage is governed by Â§7.6.1 Coverage Matrix; Phase 0 is closed and MUST NOT be used for scheduling newly discovered requirements (remediate in Phase 1 or later).
- [ADD v02.52] Implement ACE-RAG-001 as the canonical RAG contract: packs-first routing, deterministic scoring/selection, hash-key caching, and drift detection wired into Operator Consoles.
- [ADD v02.52] Add: Bundle export covers imported files + ingestion outputs in a portable, policy-safe way.

- [ADD v02.68] Enforce v1.2 evidence semantics for any D1 ingestion/verification engine: external/non-deterministic claims must carry evidence artifacts (snapshots/screenshots) and be replayable from bundles.

- [ADD v02.79] Make Photo Studio content **searchable in Shadow Workspace** (metadata + previews/proxies indexed; raw binaries remain referenced).

- **[ADD v02.130] Loom semantic enrichment**  
  Make LoomBlocks searchable and semantically enrichable: auto-tag (suggested â†’ confirmed), auto-caption for media, and hybrid retrieval over Loom content in Shadow Workspace.
- **[ADD v02.131] Stage Phase 2 â€” Mechanical feedback loops + ingestion integration**  
  Upgrade Stage capture/import jobs to integrate with Docling + Shadow Workspace (cache/index assimilation), and extend the 3D Mechanical Assist Pack with canonicalize/optimize/physics checks and reviewable reports.



**MUST deliver**

1. **Docling integration (mechanical ingestion)**  
   - Integrate Docling as described in Section 6.1.  
   - Implement the **Docling AI Job profile**:
     - Jobs for format detection, conversion, structure extraction, and error recovery.  
   - Support importing at least `.docx` and `.pdf` into internal document blocks.  
   - Provide ingestion fallbacks for unsupported formats using **Unstructured** or **Apache Tika**, especially for email containers and odd legacy documents; fallbacks must still run through the same capability/logging pathways.  
   - Log ingestion jobs and their states in Flight Recorder (including failures and retryable conditions).

   - When Docling is run as a remote or sidecar service, it SHOULD be exposed as an MCP server:
     - Run the Docling MCP server as described in Section 6.1.2.7.4 where applicable.  
     - Invoke `convert_document` (and related tools) via the Rust MCP client and Gate (Section 11.3.2), not via ad-hoc HTTP.  
   - Implement the reference-based binary protocol for Docling jobs (Section 11.3.3):
     - Use sandboxed file references/URIs for large artefacts instead of embedding base64 in MCP messages.  
     - Enforce sandbox roots and symlink protections per Section 11.3.7 when resolving these URIs.  
   - Map Docling MCP progress and logging into the existing job and logging systems:
     - Use `notifications/progress` to update ingestion job rows (Target 3).  
     - Use `logging/message` to emit ingestion metrics into Flight Recorder (`fr_events`) (Target 5).  
   - [ADD v02.36] Debug Bundle (ingestion/RAG) includes Docling tool logs/progress + artifact references (hashes/handles), plus failure diagnostics.


2. **Shadow Workspace (index + graph)**  
   - Implement the Shadow Workspace as per Section 3:
     - Incremental parsing and chunking of documents.  
     - Incremental parsing and chunking of documents using **Tree-sitter** (or equivalent) for code-aware splits.  
     - Embedding generation via a local model (default: **nomic-embed-text** for text).  
     - Storage of embeddings and metadata in a local store (use a local vector store such as **LanceDB** or **sqlite-vec**).  
     - Image embeddings captured with **CLIP** for visual assets.  
     - Provide a minimal grid/sheets surface backed by **HyperFormula** so indexed tabular data can be computed against within the Shadow Workspace.  
   - Provide a unified "Search workspace" command in the UI using Shadow Workspace.
   - Expose Shadow Workspace inspection in **Operator Consoles** (see Â§10.5):
     - **Index Doctor**: freshness/backlog metrics, rebuild/backfill actions (capability-gated), and invariant/consistency diagnostics.
     - **Descriptor/Graph Explorer (read-only)**: show indexed entities/descriptor rows with provenance and â€œwhy this was retrievedâ€ links.
  
   - Emit metrics for indexing operations and query counts; record search queries and result identifiers in Flight Recorder or a dedicated search log.
   - [ADD v02.40] Cache-to-Index Assimilation (`LocalWebCacheIndex`) as part of Shadow Workspace indexing (see Â§2.3.8.1):
     - Store external fetches used for retrieval into a local cache index; normalize (boilerplate strip + heading/anchor preservation), chunk, and hybrid-index.
     - Implement TTL + pinning (â€œgold sourcesâ€) and surface cache freshness/staleness metrics in Operator Consoles (Index Doctor).
     - Persist external fetches to `LocalWebCacheIndex` on agent stop (queue assimilation jobs if needed).

3. **RAG-aware AI jobs**  
   - New job kinds for:
     - â€œAnswer question using workspace documents.â€  
   - These jobs MUST:
     - Query Shadow Workspace for relevant chunks.  
     - Include retrieved context in prompts.  
     - Log retrieval steps and context (e.g. document IDs, snippet hashes) in Flight Recorder.  
   - Provide a debug view for at least one RAG action that shows:
   - Provide a **RAG Playground & Query Debugger** in Operator Consoles:
     - Inspect ranked chunks (IDs + hashes + ranks), prompt budget/truncation flags, and rerun retrieval without generating.
     - Deep-link from an AI answer â†’ its retrieval set â†’ the source documents/snippets.
     - [ADD v02.40] Retrieval fallback MUST consult `LocalWebCacheIndex` before external providers (cache-before-external).
     - [ADD v02.40] Retrieval MUST be snippet-first; escalate `snippet â†’ section â†’ fullpage` (fullpage = last resort; stored as artifact/cache only; never inject raw full pages into prompts); enforce per-step budgets and log truncation/compaction decisions.
     - [ADD v02.42] Implement adapter-level SEARCH â†’ READ separation: `search() -> snippets` and bounded `read(section_selector) -> excerpt` for LocalDocsIndex and LocalWebCacheIndex; section selectors must be bounded (heading/anchor ranges) and logged.
     - [ADD v02.42] Log escalation rationale when moving from `snippet` to `section` (and `fullpage` if used; why evidence was insufficient) and record `fetch_depth` explicitly on every evidence item.

     - [ADD v02.40] Evidence/provenance: capture `trust_class`, `fetch_depth`, and `cached_artifact_ref` when evidence is served from cache; surface cache-hit vs external-hit in the RAG Query Debugger.

     - Which documents/snippets were used.  
     - How they influenced the final answer (e.g. by showing context alongside the answer).
   - [ADD v02.36] Enforce evidence binding: answers must carry linked evidence refs; policy may fail the job or mark it incomplete when evidence is missing.
   - [ADD v02.36] Debug Bundle (ingestion/RAG) includes retrieval query, ranked chunk IDs/hashes, embedding/index configuration + versions, and prompt budget/truncation flags.

12. **[ADD v02.52] Workspace Bundle v0 (Expanded)**
   - Workspace Bundle v0 expands to include imported raw assets + key derived/canonical snapshots produced by ingestion.


4. **Descriptor extraction core (image + text)**  
   - Implement the DES-001 / IMG-001 / TXT-001 descriptor pipelines at MVP level for any content imported via Docling or direct file import, as defined in Sections 1.3 and 6.3.  
   - For each ingested document or asset:
     - Extract visual descriptors for images (and simple frame snapshots for video, where available) into DescriptorRows keyed by source material.  
   - Extract text descriptors for imported documents and user-authored text into TextDescriptorRows keyed by document/block (default embedding model: **nomic-embed-text**).  
   - Attach `content_tier`, `consent_profile`, and NSFW flags to each descriptor row, as defined in the Corpus/content-tier schema.  
 - Descriptor extraction MUST behave as a mechanical pipeline:
   - Jobs are AI Jobs under the Workflow Engine with job IDs, configs, status, and errors.  
   - All writes go through the DES-001 CORPUS/Sidecar contract; helpers never bypass CORPUS-DES001-NEW.  
 - Descriptor pipelines MUST respect Diary invariants:
   - Raw Corpus rows are never censored or euphemized; vocabulary control lives in CONFIG.  
   - SFW-only views and export redaction are handled in consuming views and export jobs (COR-700/701), not in extraction.
5. **Mail read-only ingestion**  
   - Mail store + IMAP/JMAP sync with `READ_EMAIL` capability; stable IDs (`internal_message_id`, `rfc822_message_id`, `provider_message_id`, `account_id`, optional `imap_uid/modseq`).  
   - Parse bodies via Unstructured/Tika; run attachments through Docling + OCR + ASR.  
   - Inject MailMessage blocks (RawContent) and TXT-001 mail descriptors (`MAIL_COMMUNICATION` domain) into Shadow Workspace; classification kept separate from other descriptor domains.  
   - Minimal UI: thread list + read-only view; FR logs engine versions/configs and ingestion results.

6. **Normalization, routing, and quality for ingestion**  
   - Indexing: declare `Indexer` as a first-class Shadow Workspace component with freshness/backlog metrics.  
   - Language/normalization: add `Detector` (language ID), `Converter` (text/encoding cleanup), and `Morphologist` (lemma/stem) stages to ingestion/descriptor pipelines.  
   - Data quality: add `Inspector` for audits/invariant checks and `Router` for explicit workflow/data-flow logging.

7. **SDXL / ComfyUI sidecar render (minimal, OSS)**  
   - Treat ComfyUI (AGPL) as a sidecar mechanical engine; do not reimplement a renderer.  
   - Provide a pinned SDXL text-to-image workflow graph (checked-in JSON) and call it via a mechanical job profile (e.g., `ATELIER_RENDER`) through the Workflow Engine; no graph authoring UI.  
   - Default model: **SDXL 1.0 base** (required) with optional SDXL 1.0 refiner; record `model_name` + SHA256 at runtime in Job History/FR. User-provided alternative models are allowed but must be logged by hash and are the userâ€™s licensing responsibility.  
   - Inputs: prompt, seed, steps, CFG scale, width/height (cap at e.g. 1024x1024), workflow_id/workflow_hash, model_hash.  
   - Outputs: store rendered images as DerivedContent with sidecar provenance (params, model/workflow hashes, ComfyUI request/response trace) and log to Flight Recorder + Job History.  
   - Governance/ops: gate via `atelier.render` capability; enforce quotas/timeouts (e.g., 120s), max output size, and VRAM guard; scrub secrets; health check ComfyUI before dispatch (local-only).  
   - UX: minimal trigger (e.g., â€œRender image from briefâ€) and artifact viewer link; show job status from Workflow Engine/Job History.  


8. **[ADD v02.44] Asset + paper ingestion as first-class Shadow Workspace sources**  
   - Implement `creative.asset_library.pipeline` ingestion (Tika + libvips + ExifTool + Czkawka) producing deterministic derived artifacts (thumbnails/metadata/dedupe).
   - Implement `science.ingest.papers_grobid` ingestion (GROBID service) producing structured paper + references for RAG.


9. **[ADD v02.47] Charts, Dashboards, and Decks (finance output MVP)**  
   - Implement `Chart` as a first-class entity that references a `Table` by ID plus optional range/query; persist only `chart_spec` (no raw table duplication).  
   - Implement dashboards as a layout/composition over existing entities (Charts + Tables + KPI blocks), typically via Canvas/Doc embedding (no new datastore).  
   - Implement `Deck` as a first-class entity whose slides reference existing entities (doc blocks, canvas frames, charts, assets).  
   - Provide in-app presenting (Reveal.js) and deterministic export (PPTX minimum) via mechanical jobs; exports produce artifact references with provenance.  
   - All non-trivial chart/deck create/update/export actions MUST run as explicit jobs under `charts_decks_ai_v0.1` (Â§2.5.11) with preview + validators.

10. **[ADD v02.49] Export + artifact discipline for Phase-2 exporters**  
   - Enforce `ExportRecord` + SHA-256 + canonical bundle hashing for any export job introduced in Phase 2 (deck_export_*, chart exports, Debug Bundle exports used by Operator Consoles).  
   - Any ingestion producing multi-file outputs (figures/tables/thumbnails) MUST emit per-artifact manifests and apply retention/pinning defaults based on classification.


13. **[ADD v02.79] Photo ingestion + catalog + indexing (Phase 2 baseline)**  
   - [ADD v02.79] Catalog/DAM baseline: collections, flags/ratings, folder sync; metadata read/write pipeline.  
   - [ADD v02.79] Proxy + preview pipeline as mechanical jobs (proxy-first for AI later).  
   - [ADD v02.79] Index photo metadata (and optionally captions) into Shadow Workspace for retrieval/debug.  



14. **Loom AI enrichment + hybrid search** [ADD v02.130]  
   - Add JobKinds + profiles for `loom_auto_tag`, `loom_auto_caption`, and `loom_batch_link` (Â§2.6.6; Â§10.12) and ensure they run through the Workflow Engine + Capability model (no bypass).
   - `loom_auto_tag`: produce `AI_SUGGESTED` edges only; provide accept/reject UI to convert to `TAG` edges per **[LM-TAG-005]**; emit FR-EVT-LOOM-008/009/010.
   - `loom_auto_caption`: generate `LoomBlock.derived.auto_caption` for image/video/audio assets; store as DerivedContent with attribution.
   - Enable Tierâ€‘3 Loom search (semantic/hybrid) by generating embeddings for LoomBlocks and wiring them into Shadow Workspace hybrid query path (Â§2.3.14.8.1).
   - Batch link suggestion (`loom_batch_link`) MAY propose `AI_SUGGESTED` edges for user review (no silent writes).

15. **Handshake Stage Phase 2 upgrades (ingestion + 3D feedback loops)** [ADD v02.131]  
   - Upgrade `stage.import_pdf.v1` to use the Docling conversion pipeline (Â§6.1) and emit structured document blocks + descriptors; original bytes remain preserved as artifacts.  
   - Add Cache-to-Index Assimilation for Stage captures: captured `artifact.snapshot` bundles can be normalized/chunked and indexed into `LocalWebCacheIndex` + Shadow Workspace (cache-before-external for later retrieval).  
   - Extend the 3D Mechanical Assist Pack with deterministic feedback loops: `stage.3d.canonicalize_gltf.v1`, `stage.3d.optimize_mesh.v1`, and `stage.3d.physics_checks.v1`, producing reviewable reports + artifacts (no silent mutation).  
   - Deliver at least one Stage App for review: scene/constraint report viewer (3D QA) and an evidence browser that deep-links Stage outputs (snapshots/clips/reports) to their jobs + provenance.  


- [ADD v02.137] **Design Studio multi-session support:**
  - Use `target_entity_ids` on `ModelSession` to bind sessions to CRDT-backed entities (canvas/doc/table).
  - Define entity-level locking discipline (Phase 2): locks are per-entity or per-node; a session can hold multiple locks; locks are released on session completion or timeout.
  - Expose "entity lock state" in the DCC Sessions panel for Design Studio workflows.
  - Add a â€œparallelizeâ€ action in Design Studio that spawns child sessions for sub-entities (copy/layout/images).

**Vertical slice**  
- Import a `.docx` or `.pdf` file.  
- Wait for ingestion to complete and open the resulting document.  
- Use "Search workspace" to find content from the imported file.  
- Ask a question that should be answered from that content; see a RAG-backed answer and inspect the corresponding jobs and logs (ingestion + retrieval + answer job).
- Open a descriptor/debug view for the imported file and inspect at least one DescriptorRow (image or text) with correct `content_tier` / `consent_profile` metadata and provenance.
- [ADD v02.40] Run one external web retrieval (allowed/consented), then repeat the same question and confirm it is answered from `LocalWebCacheIndex` (cache hit visible in RAG Debugger; no second external call).
- [ADD v02.47] Import a financial PDF â†’ extract a table â†’ create a chart referencing that table â†’ assemble a 3â€‘slide deck (title + chart + table) â†’ export PPTX â†’ verify provenance in Job History + Flight Recorder.
- [ADD v02.52] Import a PDF/DOCX, run ingestion, export Workspace Bundle; verify original bytes + canonical snapshot + Display-derived render included.


11. **[ADD v02.52] ACE-RAG-001 Retrieval Contract (RAG correctness, speed, token efficiency)**
   - Implement `ContextPack` builder job (`context_pack_builder_v0.1`) producing pack artifacts with `facts/constraints/open_loops/anchors/coverage` and SourceRefs.
   - Enforce pack freshness (`ContextPackFreshnessGuard`) using source hashes; stale packs MUST not be preferred.
   - Implement `IndexDriftGuard`:
     - Embedding drift: vector/snippet records carry `source_hash`; mismatch triggers drop/downgrade + reindex recommendation.
     - KG drift: candidates used as evidence require provenance; missing provenance disqualifies evidence use.
     - LocalWebCacheIndex drift: TTL/pinning warnings surfaced; pinned-but-stale marked clearly in traces.
   - Implement hash-key caching for retrieval stages:
     - retrieval candidate list cache (cache_kind=`retrieval_candidates`)
     - rerank order cache (cache_kind=`rerank_order`)
     - (optional) spans cache and prompt envelope cache once stable
   - Determinism split:
     - strict mode: deterministic ranking + tie-breaks; deterministic rerank only.
     - replay mode: persist candidate ids/hashes + rerank order; replay reuses persisted order.
   - Upgrade Operator Consoles RAG Query Debugger + Index Doctor to show:
     - QueryPlan + RetrievalTrace ids/hashes, route taken, cache hits/misses, candidates + scores + tie-break keys,
       selected spans + truncation flags, drift flags, and degraded-mode markers.

- [ADD v02.52] Verify ContextPacks are preferred when fresh, fall back is logged when stale; RAG Debugger shows QueryPlan/Trace, cache hit/miss, and drift flags for the answer.



12. **[ADD v02.138] Front End Memory System (FEMS) v1 (hybrid retrieval + pack governance)**  
   - Enable hybrid retrieval over `MemoryItem`s (FTS + vector + graph) with deterministic selection and replay logging (Â§2.6.6.7.6.2).  
   - Enforce per-type quotas (intent/risk/tool_protocol/etc.) and strict scope matching (workspace/project/WP).  
   - Add consolidation + conflict workflows surfaced in DCC Memory Panel; merges are `supersede`/`merge` operations with versioned history.  
   - Support optional precomputed per-WP `MemoryPack`s and pack invalidation on memory commit events.  
   - Extend cloud redaction rules for memory packs and record decisions in `ContextSnapshot`.

13. **[ADD v02.67] Atelier Lens at scale (Role lanes + organic growth controls)**
   - Implement `ATELIER_LANE_INDEX` to build role-scoped retrieval lanes (lexical + vector) over:
     - `RoleDescriptorBundle` (role overlays, evidence-linked)
     - `RoleDeliverableBundle` (typed outputs)
   - Implement organic growth queue jobs:
     - `ATELIER_VOCAB_PROPOSE` (emit proposed terms/fields with example evidence)
     - `ATELIER_VOCAB_RESOLVE` (accept/merge/reject; produces a new vocab snapshot id)
   - â€œSearch as roleâ€: add retrieval routing so role lanes are queryable and preferred when a role lens is selected.
   - Scheduling: enforce budgets so only top-k roles run deep extraction; `RoleGlance` remains cheap and non-blocking at corpus scale.
- [ADD v02.79] Import RAW+JPEG set â†’ generate previews/proxies â†’ search by metadata â†’ open from results â†’ export derivative; confirm provenance chain.



- **Loom AI loop** [ADD v02.130]
  - Select a LoomBlock (media or document) and run `loom_auto_tag`.
  - Verify suggested tags appear as suggestions (DerivedContent) without altering RawContent.
  - Accept one suggested tag: confirm it becomes a `TAG` edge and appears in backlinks/tag hub.
  - Reject one suggested tag: confirm it is removed and recorded.
  - Run Loom search with semantic/hybrid enabled and confirm results include the tagged/captioned item.

**Key risks addressed in Phase 2**
- [ADD v02.130] Loom AI trust risks: suggested tags/captions must be clearly labeled as DerivedContent, reversible, and never silently mutate RawContent; acceptance must be explicit and logged.
- [ADD v02.138] FEMS scale + privacy risk: hybrid memory retrieval can reintroduce drift or leak sensitive context unless bounded and consent-gated. Mitigation: per-type quotas, strict trust/truth gating, cloud redaction rules, DCC review, and `FR-EVT-MEM-*` auditing.


- [ADD v02.52] Ingested content cannot be backed up/moved while preserving provenance/IDs.
- [ADD v02.47] Charts/decks accidentally become a parallel datastore (data copied into chart/deck content instead of ID-based references).
- [ADD v02.47] Export provenance gaps (missing hashes/engine version/policy) prevent reproducibility and audit.
- [ADD v02.47] Export policy leakage: sensitive content exported without `SAFE_DEFAULT` gating and explicit logging.

- [ADD v02.49] Bundle hashing nondeterminism makes exports unverifiable across runs/devices; mitigate with canonical bundle hashing + recorded content_hash and per-file manifests.

- Ingestion pipeline is too brittle or slow to be practical.  
- Shadow Workspace design is wrong or too hard to debug.  
- RAG behaviour is opaque (user cannot see why an answer was produced).
- Descriptor pipelines drift from DES-001 / TXT-001 / IMG-001 law or accidentally censor/soften internal Corpus.
- Language/normalization gaps or missing audits make search/RAG results untrustworthy.
- MCP-based ingestion (Docling or other engines) diverges from non-MCP paths, leading to inconsistent capability enforcement, logging, or provenance between tool interfaces.
- [ADD v02.40] Cache growth + staleness in `LocalWebCacheIndex` leads to wrong answers or citation rot (mitigate via TTL + pinning + refresh + staleness surfacing).
- [ADD v02.40] Low-trust sources poison the local cache and outrank authoritative docs (mitigate via trust classification + downranking + cross-verification for high-impact outputs).
- [ADD v02.40] Allowlist crawling / offline mirroring violates ToS/licensing if misused (mitigate via explicit allowlists + conservative defaults + audit logs).
- [ADD v02.40] Privacy leakage: cached pages contain sensitive material that is later exported or sent to external models (mitigate via consent gates + minimization + sensitivity tagging + never writing redactions back into stored content).


8. **Calendar ingestion and ICS invite pipeline (read-only external, local drafts)**
   - Implement `calendar_sync` as a mechanical engine for **read-only** provider ingestion (CalDAV) into a local calendar store with idempotency keys and observable sync state.
   - Parse **ICS attachments** from mail ingestion into **draft** CalendarEvents (no external export) and attach provenance to the source MailMessage/thread.
   - Store and surface timezone/recurrence fields as Raw/Derived data (no advanced UI editing required in this phase).
   - **[ADD v02.63] Full Calendar Law delivery (write + recurrence + governance):** add recurrence editing UI, patch-set mutation governance (expected_etag/local_rev + provenance), identity/idempotency rules, and export/write policy enforcement aligned with AÂ§10.4.1.
   - **[ADD v02.63] ACEâ†”Calendar compatibility:** add ACE compatibility tests (scope hint, cache/prefix stability) for calendar context; ensure calendar writes/reads do not violate ACE determinism/caching rules.


- [ADD v02.44] JVM-based services (Tika/GROBID) increase packaging/ops complexity and require strict resource limits and isolation.
- [ADD v02.44] Untrusted file parsing (PDF/media) is a high-risk surface; enforce Â§11.7.4 untrusted-input policies and capture evidence/provenance for every derived artifact.
- [ADD v02.131] Stage 3D transforms risk: canonicalize/optimize/physics checks can introduce silent geometry drift unless treated as deterministic jobs with before/after hashes + diffable reports; no write-back to source assets without explicit approval and logged provenance.
- [ADD v02.67] Role-lane indexing and â€œorganic growthâ€ can degrade queryability if uncontrolled; mitigated by proposal queues, vocab snapshot ids, and lane rebuilds driven by accepted changes.
- [ADD v02.67] Lane/index drift breaks replayability; mitigated by pinned lane build configs + hash-keyed rebuild semantics and surfacing drift in Inspector/Operator Consoles.
- [ADD v02.67] Compute cost grows superlinearly with corpus size; mitigated by claim/glance/deep split with strict budgets and backpressure-aware scheduling.


- [ADD v02.68] External facts are not verifiable/replayable (link rot, changing pages, disputes); mitigated by Archivist/Guide evidence bundles and v1.2 D1 evidence requirements (replay uses captured artifacts).
- [ADD v02.79] Index bloat / perf regression â†’ index references + derived previews only (artifact-first), not raw pixels.


**Acceptance criteria**
- [ADD v02.130] Loom AI: auto-tag produces AI_SUGGESTED edges; accept/reject converts or removes; captions stored as DerivedContent with attribution; semantic/hybrid Loom search returns expected results and is observable via Flight Recorder.
- [ADD v02.138] FEMS v1: hybrid memory retrieval produces bounded `MemoryPack`s with quotas; DCC shows pack preview + memory review queue; commits emit `FR-EVT-MEM-*`; replay reproduces pack hash; consolidation produces supersedence history (no silent overwrites).
- [ADD v02.131] Stage PDF import: `stage.import_pdf.v1` runs through the Docling/MCP path and produces structured doc blocks + descriptors; original PDF bytes remain preserved as an artifact; job is inspectable in Job History/Flight Recorder.
- [ADD v02.131] Stage capture assimilation: captured `artifact.snapshot` bundles become searchable via `LocalWebCacheIndex`/Shadow Workspace and can satisfy later retrieval with stable artifact refs (no second external fetch when cache is fresh).
- [ADD v02.131] Stage 3D feedback loops: `stage.3d.canonicalize_gltf.v1`/`stage.3d.optimize_mesh.v1`/`stage.3d.physics_checks.v1` produce deterministic reports with before/after hashes; Stage App can review reports and any write-back requires explicit approval + logged provenance.


- [ADD v02.47] Chart stores only spec + entity refs; table edits update render; no duplicated table rows/cells exist in Chart RawContent.
- [ADD v02.47] Deck export produces artifact references only and records: deck_id/slide_ids, referenced entity IDs + hashes, export engine + version, and export policy.
- [ADD v02.47] Chart/deck create/update/export operations are visible as explicit jobs (with previews, validators, and Flight Recorder traces).

- At least one external calendar can be ingested read-only via `calendar_sync` and rendered in the Calendar surface with provenance and sync diagnostics visible.
- **[ADD v02.63] Calendar Law compliance tests pass (AÂ§5.4.6.4) and ACEâ†”Calendar compatibility tests added; recurrence editing/write governance enforced per AÂ§10.4.1 (patch-set, idempotency, identity).**
- **[ADD v02.63] Contextual hardening primitives:** audit trail and retention policies are recorded (TraceRetentionPolicy/AuditTrail/CapabilityGrant logs); capability grants/denials and retention/redaction defaults are visible in FR/Problems; schema/docs updated for these primitives.
- Ingestion jobs are visible and inspectable in Job History and Flight Recorder.
- Shadow Workspace can be inspected via logs or a debug view (e.g. number of indexed documents, last index time).
- Shadow Workspace inspection is available via Operator Consoles (Index Doctor) and supports rebuild/backfill with FR+Problems linkage.
  
- For at least one RAG scenario, you can show which documents and snippets were used to produce an answer.
- RAG Query Debugger can show the ranked retrieval set and prompt-budget/truncation decisions for that answer.
- [ADD v02.40] After any external fetch, the cached artifact becomes locally searchable and can satisfy subsequent retrieval without another external call (cache hit visible in RAG Debugger).
- [ADD v02.40] TTL + pinning behavior is testable: pinned sources survive eviction; expired sources are marked stale; refresh path works under consent policy.
- [ADD v02.40] Evidence logs include `trust_class`, `fetch_depth`, and `cached_artifact_ref` for cached sources; retrieval logs record cache-hit vs external-hit.
- [ADD v02.42] RAG Query Debugger surfaces per-item `fetch_depth` and escalation rationale; section reads are bounded to heading/anchor ranges with stable citations/SourceRefs.


- At least one Docling ingestion job runs via MCP (server or sidecar) end-to-end, with progress updates and `logging/message` events visible in Job History and Flight Recorder, and consistent provenance/capability metadata.
- Descriptor extraction jobs appear in Job History and Flight Recorder with clear linkage to source documents/assets.  
- For at least one imported file, descriptor rows can be inspected from a debug view or console query, showing stable IDs, schema versions, and consent/content-tier fields.  
- A SFW-only workspace/search mode can be toggled to filter NSFW descriptors without modifying underlying Corpus rows.
- Shadow Workspace under churn shows index freshness/backlog metrics; rebuild/backfill command succeeds.  
- RAG jobs log retrieved snippet IDs/hashes and prompt budget/truncation flags.  
- Descriptor law enforced: sampled descriptors show schema version, consent/content-tier, nsfw flag, sidecar-only provenance; unit/physics middleware normalizes sheets/docs ingestion.  
- Monaco initial gating: LSP/diagnostics routed through capability-aware gates; worker routing configured; FR logs model-assisted code actions.  
- Physics/dimension_check validator enforced in Sheets/Docs: unit errors flagged, normalized values returned, FR logs validator outcomes.  
- Wrangler/DBA outputs open in Sheets/Canvas grid views; FR entries carry doc/table/entity references.  
- Mail ingestion: fixture sync shows MailMessages with correct IDs/thread linkage; attachments processed (Docling/OCR/ASR) with FR provenance; TXT-001 mail descriptors stored under `MAIL_COMMUNICATION`; `READ_EMAIL` enforced/FR-logged; health check exposes latest modseq/sync time.
- Normalization/quality: language tags recorded (Detector), encoding cleanup applied (Converter), lemma/stem normalization available (Morphologist), audits and workflow routing logged (Inspector/Router).
- SDXL/ComfyUI render: given a local ComfyUI server and the pinned SDXL graph, a render job runs via Workflow Engine, is visible in Job History/FR with model/workflow hashes and params, is capability-gated, enforces timeout/output caps, passes health check, and produces a stored artifact with sidecar provenance (hashes, params, request/response trace).  
- [ADD v02.36] Debug Bundle for Docling/RAG runs includes ingestion logs + indexing config + retrieval set (IDs/hashes) + model/version metadata, redacted-by-default.
- [ADD v02.36] RAG jobs enforce evidence binding per policy (fail or mark incomplete) and surface the reason in Problems/Evidence.


- [ADD v02.44] Mixed corpus ingest (scientific PDF + images) produces Shadow Workspace entities and RAG citations with provenance back to source artifacts.
- [ADD v02.44] Asset ingestion produces deterministic derived artifacts (thumbnails/metadata sidecars) and dedupe results stored as job artifacts.
- [ADD v02.49] Deck export bundles include canonical bundle hash and per-file manifests (or embedded `bundle_index.json`); ExportRecord includes export policy, engine/version/config hash, and SHA-256 hashes.
- [ADD v02.49] Ingestion-derived artifacts (tables/figures/thumbnails) can be pinned and survive TTL/GC; retention state is inspectable via Operator Consoles.

- [ADD v02.52] ACE-RAG-001 conformance tests pass (T-ACE-RAG-001..007): normalization determinism, strict ranking determinism, replay persistence, pack freshness invalidation, budget enforcement, drift detection, cache invalidation.
- [ADD v02.52] Repeating the same query over unchanged sources shows cache hits in RetrievalTrace and reduced retrieval latency (measured in Flight Recorder).
- [ADD v02.52] RAG Debugger deep-links: Answer â†’ RetrievalTrace â†’ selected spans â†’ source documents; every evidence item is bounded and provenance-carrying.
- [ADD v02.52] export_report lists inclusions/exclusions and reasons; denials visible in Problems + Flight Recorder.
- [ADD v02.52] exported entities preserve stable IDs referenced by jobs/workflows.


- [ADD v02.67] Role-lane search exists: selecting a role lens routes queries through the correct lane index, and results show evidence refs and provenance.
- [ADD v02.67] Vocab/schema proposal queue works end-to-end (propose â†’ review/resolve â†’ new snapshot id â†’ lane rebuild) and produces audit logs in Flight Recorder/Operator Consoles.
- [ADD v02.67] Lane rebuilds are deterministic under pinned configs and surface drift flags when underlying sources change.
- [ADD v02.79] Photo search returns correct assets by metadata/collection; opening a result shows traceable derivation artifacts.


**Explicitly OUT of scope**
- [ADD v02.130] Loom: real-time multi-user collaboration, cross-device sync semantics, and Postgres-backed Loom view engines remain Phase 4.
- [ADD v02.131] Stage: full Stage Studio authoring and advanced 3D editing (Spline-class), third-party Stage App marketplace/extensions, and cross-device/multi-user Stage sessions remain Phase 3+ / Phase 4.


- [ADD v02.47] Extension marketplace for chart/deck templates, thirdâ€‘party exporters, and collaborative review/commenting (defer to Phase 4).

- **[ADD v02.63] External calendar provider write-back (CalDAV PUT/DELETE) remains out-of-scope; Phase 2 covers local recurrence/editing only.**
- Advanced knowledge graph visualization.
- Complex retriever configuration UIs.
- Graph/node authoring UI for image workflows, advanced ComfyUI graphs, or multi-model routing; only the pinned SDXL workflow is in scope.  
- Taste Engine training and symbolic profiles (SYM-001); Phase 2 only requires raw descriptor extraction and wiring.
- ASR transcription capabilities (e.g., Whisper/FFmpeg) and related media pipelines.
- Advanced multi-agent orchestration beyond basic agent invocation (e.g., AutoGen/LangGraph multi-step flows).
- Full mail client functionality beyond read-only ingestion and rendering.
- [ADD v02.40] Bulk crawling/archiving of arbitrary websites beyond explicit allowlist runs (no â€œmirror the webâ€ mode).


- [ADD v02.52] Cross-device/multi-user caching and pack-regeneration policy (collaboration-safe semantics) â€” Phase 4.
- [ADD v02.52] â€œRehydrate full index from bundleâ€ as a supported workflow (future phase).
- [ADD v02.52] publishing bundles as shareable links.

- [ADD v02.79] Full layer compositor, advanced masking, merges, AI generation.


**Mechanical Track (Phase 2)**
- [ADD v02.130] Loom Tierâ€‘3 index build: embedding generation jobs for LoomBlocks + incremental refresh; hybrid retrieval wiring in Shadow Workspace.
- [ADD v02.131] Stage ingestion + 3D mechanicals: Stage snapshot normalization/index jobs (Archivistâ†’Indexer) and 3D assist engines (canonicalize/optimize/physics) run via Tool Bus with deterministic outputs, before/after hashes, and evidence artifacts; Stage App review required for any asset mutation.
- [ADD v02.136] **Handshake as MCP server (local)**: expose a curated subset of Tool Registry tools over MCP for cloud-model orchestration. MUST be capability-scoped, secret-safe (redaction), and Tool-Gate enforced.
- [ADD v02.136] **Tool Search / deferred loading** (Â§6.0.2.7): implement `handshake.tools.search`/`handshake.tools.get` backed by Tool Registry tool packs to reduce context bloat as tool count grows.


- [ADD v02.47] Deck export mechanical jobs: `deck_export_pptx` (PptxGenJS) and optional `deck_export_pdf`/`deck_export_html`; capability-gated; Flight Recorder logs artifact hashes + engine/version + policy.
- [ADD v02.47] Chart/deck validators as mechanical gates: `chart_spec_schema`, `no_parallel_store`, `artifact_reference_only`, and `export_policy_gate` enforced and logged.
- Ingestion-focused engines: `Archivist` (SingleFile/yt-dlp), `Librarian` (metadata/BibTeX/EXIF), `Wrangler` (Great Expectations/csvkit), `DBA` (DuckDB/sqlite-utils/Tantivy), `Indexer` (Shadow Workspace indexing), `Router` (workflow data-flow orchestration), and `Inspector` (data audits).
- Descriptor/normalization engines: wire IMG-001 / TXT-001 helpers, add `Converter` (text/encoding normalization), `Morphologist` (lemma/stem), and `Detector` (language ID) into the mechanical runner with DES-001 CORPUS/Sidecar and capability gates.
- Add unit/physics middleware for sheets/doc ingestion to enforce unit consistency and conversions.
- Acceptance: ingestion/descriptor/indexing runs show mechanical provenance in Flight Recorder; Shadow Workspace metrics/logs expose indexed docs and language/normalization steps; RAG answers list retrieved snippets and source artifacts; data audits (Inspector/Router) log routing decisions and invariants.
- When these engines are exposed as remote services, they SHOULD prefer MCP as the tool interface and MUST route calls through the same MCP Gate and Flight Recorder paths as other AI jobs (Sections 2.6.6 and 11.3).
- [ADD v02.40] Implement Cache-to-Index Assimilation as a mechanical job using existing engines (`Archivist` capture â†’ `Indexer` normalize/chunk/index â†’ `Inspector` TTL/pinning audits) and log all steps in Flight Recorder.
- [ADD v02.52] `workspace_bundle_export_v0` supports inclusion of imported raw assets + selected derived sidecars (policy-gated).
- **[ADD v02.63] OS primitives track (Window/Shell Engines):** add UI automation and sandboxed shell mechanical jobs (capability-gated, logged, bounded outputs) with FR/Job History visibility and health/timeout/quotas.

- [ADD v02.115] **MT Executor context compilation engine:** Implement MT context compilation as mechanical job using ACE Runtime (Â§2.6.6.7) + ContextPacks (Â§2.6.6.7.14.7) for deterministic, bounded context assembly per iteration.
- [ADD v02.115] **MT validation command registry:** Extend validation command allowlist with project-type inference (Cargo.toml â†’ cargo check/test, package.json â†’ npm test, etc.); log all validation runs via Mechanical Tool Bus.

- [ADD v02.115] **AI-Ready Data Architecture (Â§2.3.14) - Phase 2:**
  - Implement two-stage retrieval with cross-encoder reranking (25-48% ranking improvement)
  - Implement parent document retrieval (retrieve small chunks, expand to larger context)
  - Implement contextual prefix generation (LLM-generated explanatory text, 35-67% failure reduction)
  - Integrate Knowledge Graph relationships (18 types per Â§2.3.14.6) with hybrid search
  - Implement cross-modal embedding (CLIP for images) for Photo Studio integration
  - Implement context pollution scoring and alerts (FR-EVT-DATA-011)
  - Wire data validation jobs (`ingestion_validator_v1`, `retrieval_quality_audit_v1`) to Mechanical Tool Bus
  - Implement golden query testing for quality regression detection
  - Acceptance: reranking visible in retrieval traces; Photo Studio images searchable via text; pollution alerts trigger on threshold breach

- [ADD v02.116] **Locus Work Tracking System (Â§2.3.15) - Phase 2:**
  - Implement hybrid search (vector HNSW + keyword BM25 + graph traversal) for WP/MT search (Â§2.3.15.7)
  - Implement Calendar policy integration: policy profiles modulate locus_query_ready results (Focus Mode, Sprint Mode, etc.) (Â§2.3.15.4)
  - Implement dependency graph queries: getDependencyTree, getBlockingChain with O(1) Knowledge Graph traversal (Â§2.3.15.7)
  - Implement WP priority escalation based on Calendar/context signals
  - Implement migration tools: import from external systems (GitHub Issues, etc.) with metadata preservation
  - Wire to Operator Consoles: Locus Query Explorer with dependency visualization, MT execution timeline viewer
  - Extend Flight Recorder analytics: WP completion velocity, MT iteration histograms, escalation pattern analysis
  - Acceptance: hybrid search returns semantically relevant WPs; Calendar policy boosts prioritized work; dependency graph queries complete <100ms; Operator Console exposes Locus metrics




- [ADD v02.44] Creative asset ingestion jobs: thumbnails (libvips), metadata extraction (Tika + ExifTool), dedupe scanning (Czkawka) with provenance sidecars.
- [ADD v02.44] Bibliography primitives + tooling integration: `science.primitives.bibliography` (JabRef / Hayagriva / Citation.js; citeproc-rs only with MPL obligations tracked) and coupling to paper ingestion outputs.
- [ADD v02.44] Local analytics substrate + unit validator: `science.analytics.local` (Arrow + DuckDB) and `science.validators.units_dimension` (Pint) as gates for derived tables/params.
- [ADD v02.67] Implement `ATELIER_LANE_INDEX`, `ATELIER_VOCAB_PROPOSE`, and `ATELIER_VOCAB_RESOLVE` as mechanical job profiles (capability-gated; logged; deterministic outputs via snapshot ids).
- [ADD v02.67] Add lane/index Inspector audits (coverage, drift, rebuild triggers) and render lane status in Operator Consoles.


- [ADD v02.68] Apply v1.2 determinism/evidence policy: `Archivist` and `Guide` operations classified as D1 MUST emit evidence artifacts; conformance harness includes evidence-required checks and replay expectations.
- [ADD v02.79] Implement `engine.raw_decode` + minimal `engine.photo_develop` CPU path behind tool bus (pinned versions; determinism class recorded).


**Atelier Track (Phase 2)**
- [ADD v02.67] Expand the Role Registry materially (still contract-first); add at least one additional Finishing role and one â€œculture/contextâ€ advisor role as dual contracts.
- [ADD v02.67] Implement one real Nested Production dependency chain as a production graph fixture (e.g., `graphic_design` produces a pattern deliverable consumed by `fashion.styling` synthesis), and ensure `ATELIER_GRAPH_SOLVE` schedules it deterministically.
- [ADD v02.67] Add role-lens retrieval UI: â€œSearch as roleâ€, lane selection, and per-role result explanation (why this role claimed it + evidence refs).
- [ADD v02.79] Add collections/smart filters UI + search entrypoint wired to Shadow Workspace results.


- [ADD v02.115] **MT smart drop-back:** Implement "smart" drop_back_strategy that considers LoRA training history, failure category patterns, and remaining MT complexity before returning to smaller models.
- [ADD v02.115] **MT ContextPacks integration:** Wire MT context compilation to use ContextPacks (Â§2.6.6.7.14.7) for efficient file retrieval with staleness detection.
- [ADD v02.123] Implement **Tier2 auto-when-idle** scheduling for Lens extraction: Tier2 deep passes MUST auto-trigger when the app is idle (and within configured GPU/CPU budgets), queue jobs deterministically, and emit provenance + evidence. Tier1 remains default and synchronous.
- [ADD v02.123] Implement **Facts substrate** for Lens: normalized `AtelierFact` + `AtelierSymbolFact` as first-class indexed records (global across projects, filterable) with EvidenceRef pointers and deterministic retrieval traces.
- [ADD v02.123] Implement **SymbolismProfile + SymbolLexiconSnapshot** persistence and drift-safe evolution (branchable profiles): symbolic templates MUST grow over time; unknown/unclear fields MUST be stored explicitly as `unclear`/`not_available` (never omitted silently).
- [ADD v02.123] Implement Lens **global filter UI** (role filters, source filters, ViewMode toggle default NSFW, projection markers) and an Operator Console QueryPlan/RetrievalTrace viewer for Lens queries (debuggable click-to-source).
- [ADD v02.123] Implement Lens persistence contract extensions in SQLite (and migration notes for Postgres later): deterministic artifact layout for Derived Fact stores, schema versioning, and replayable QueryPlan/Trace storage.

**Distillation Track (Phase 2)**
- Persist Skill Bank entries from real workflows; implement sample selection and teacher-eval jobs flowing through Workflow Engine.
- Define and version the distillation eval suite; log pass@k/compile/test and collapse indicators; no student promotion yet.
- Acceptance: Skill Bank artifacts stored with provenance and export controls; eval runs visible in Flight Recorder with required metrics and lineage fields.
- [ADD v02.157] Adapter-only LoRA / QLoRA / DoRA training remains benchmark-gated: rank/alpha/repeats/epochs, adapter merge posture, and rollback lineage MUST be recorded as first-class evaluation metadata before any promotion path is allowed.



- [ADD v02.115] **[REMEDIATION] MT Executor LoRA feedback loop (Â§2.6.6.8.8):** Aggregate escalation_records by (lora_id, failure_category, task_tags); update lora_registry weak_on_task_types and training_priority; trigger automated distillation jobs when sufficient candidates accumulate. (Extends existing Skill Bank schemas.)
- [ADD v02.115] **MT smart drop-back implementation:** Implement ShouldDropBack algorithm checking failure_category, lora_training_updates, remaining_mt_complexity; record drop-back decisions with outcomes for threshold refinement.
- [ADD v02.115] Acceptance: LoRA training priority updates visible in Skill Bank; drop-back decisions logged (FR-EVT-MT-014); at least one LoRA improvement cycle demonstrated end-to-end.
- [ADD v02.79] Preset/recipe serialization rules finalized enough for â€œshareable presetsâ€ (still single-user).

### 7.6.5 Phase 3 â€” ASR & Long-Form Capture

**Goal**  
Add lecture/meeting capture via ASR, using the same AI Job, workflow, and observability primitives. Debugging ASR behaviour must be possible from logs and UI.

- [ADD v02.186] KERNEL-004 lands the Kernel-V0-Substrate fold: sandbox 3-adapter trait, ModelRuntime + 8 PRODUCTION inference techniques, Memory V0+ self-improvement loop on HBR test-packet corpus, process ownership ledger, visual debugger, non-hijacking GUI invariants.

- [ADD v02.105] Phase coverage is governed by Â§7.6.1 Coverage Matrix; Phase 0 is closed and MUST NOT be used for scheduling newly discovered requirements (remediate in Phase 1 or later).
- [ADD v02.52] Extend ACE-RAG-001 evidence selectors to transcripts (time-range SourceRefs), so Q&A over ASR outputs uses the same budgets/traces/drift guards.

- [ADD v02.68] Media mechanical jobs (Director/Composer) adopt v1.2 determinism classes (D2/D3 when possible) and emit conformance vectors + provenance so renders can be re-run and compared.

- [ADD v02.79] Introduce **vision-derived metadata** for photos using the same provenance/drift discipline Phase 3 applies to other stochastic outputs.
- **[ADD v02.131] Stage Studio baseline (3D provenance review)**  
  Extend Stage from capture-only into provenance-first 3D review: deterministic analysis/validation/canonicalization jobs produce reports and patch sets; a Stage App can review/apply patch sets via governed workspace mutations (no silent asset edits).



- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**  
  Validate `settings.exec_policy` end-to-end on at least one **local** runtime, including downgrade semantics and Flight Recorder observability, and run initial approximate experiments under waiver control.

**MUST deliver**

1. **ASR engine integration**  
   - Integrate an ASR engine (e.g. Whisper / whisper-rs) as described in Section 6.2.  
   - Use an optimized runtime (e.g., **Faster-Whisper** or **whisper-rs/whisper.cpp**) for batch transcription of locally stored audio/video files.  
   - Normalize media via **FFmpeg** (audio extraction, resample, channel/bitrate caps) before ASR.  
   - Log ASR runs with duration, model, and basic quality-related metrics (where available) into Flight Recorder or a dedicated ASR log.

2. **ASR AI Job profile**  
   - Implement the ASR profile:
     - `media_id`, time ranges, diarization flags, language configuration, provenance.  
   - ASR jobs MUST flow through the Workflow Engine and log into Flight Recorder like any other AI job.  
   - Expose ASR-specific status and errors (e.g. decoding failure, unsupported format) in Job History.

3. **Transcription UX**  
   - "Transcribe file" flow:
     - Drop audio/video into Handshake.  
     - See job progress and final transcript document with segments and timestamps.  
   - Transcripts MUST be regular workspace documents, subject to the same governance and editing rules as other documents.  
   - Provide at least one debug surface that shows:
     - Segment-level timeline (timestamps, diarization labels where present) and a way to open the related Flight Recorder slice.
     - A one-click Debug Bundle export for the ASR job (media hash + segment ranges + ASR config + diagnostics), redacted-by-default.

     - Input file details.  
     - Chosen model and parameters.  
      - Any segmentation or diarization decisions, where applicable.
   - (Optional) Diarization: integrate **pyannote.audio** (or equivalent) as an opt-in stage; outputs recorded as overlays/metadata, not mutating base transcripts.

4. **ASR/multi-agent orchestration (minimal)**
   - For long-form or multi-step ASR flows, allow orchestration via a minimal multi-agent framework (e.g., AutoGen or LangGraph) to manage chunking, retries, and QC.
   - All orchestrated steps MUST still run through Workflow Engine + Flight Recorder; no direct model calls.
5. **Mail AI jobs and drafting/sending**
   - Local-only jobs: `mail_summarize_thread_v0.1`, `mail_triage_inbox_v0.1`, `mail_thread_to_doc_v0.1` using local models.  
   - Drafting: `mail_draft_reply_v0.1` + mechanical `email_send` engine with `from_identity`, pre-send checks (Red Pen, Anonymizer, classification validation), before/after diff, provenance.  
   - Capabilities: `SEND_EMAIL` required; `require_confirmation = true` for AI send flows; policy-based routing for local vs cloud models (default local-only).

   - [ADD v02.138] CRM/Contact profiles (minimal): introduce `ContactCard` + contact-scoped `MemoryItem`s (`relationship_note`, `preference`) for mail drafting; default to local-only; require explicit consent/redaction for any cloud-bound prompts.

6. **NLP overlays and curation (derived-only)**
   - Add `Aligner` (parallel text), `Lexicographer` (dictionary/thesaurus), and `Curator` (collections/playlists) as Workflow Engine jobs producing DerivedContent overlays only (no schema changes to descriptors).  
   - Provenance: log inputs/outputs in Flight Recorder and attach overlays to source documents/descriptors by reference with capability gates.



7. **[ADD v02.47] Transcript â†’ Deck summary (reuse Decks)**  
   - Add a `transcript_to_deck` AI job that generates a deck from a transcript doc (agenda, key points, action items).  
   - Export via the same `deck_export_*` mechanical jobs; record provenance and policy like other exports.

8. **[ADD v02.49] ASR artifact discipline (manifests + retention + bundle hashing)**  
   - ASR outputs (transcripts, segments, debug bundles) MUST emit artifact manifests with SHA-256, respect TTL/pinning defaults, and use canonical bundle hashing for exported Debug Bundles.


9. **[ADD v02.79] Photo Vision v0 (metadata-only)**  
   - [ADD v02.79] `engine.vision` jobs: `tag`, `describe`, `analyze_quality`, optional `ocr`; outputs stored as `.hs.ai.json` artifacts with model/version/params recorded.  


10. **[ADD v02.131] Stage Studio baseline (provenance-first 3D review)**  
   - Provide a Stage Studio review surface (Stage App) that can open 3D assets + reports, show `scene_ir` diffs, and review/apply patch sets (workspace mutations only via `stage.workspace.applyPatchSet`).  
   - Add a governed job `stage.3d.apply_patchset.v1` that takes (source asset, patchset artifact) and produces a **new derived asset** (no in-place mutation) with before/after hashes + provenance + evidence bundles.  
   - Add evidence bundle export for 3D jobs (inputs, reports, patchset, derived outputs, hashes) suitable for replay and audit.

- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Implement a local runtime adapter that accepts `exec_policy` and emits `llm_exec_policy` (FR-EVT-LLM-EXEC-001) with requested vs effective policy.  
  - Add at least one approximate execution implementation for local models (e.g., LayerSkip / early-exit), gated behind waiver + Work Profile toggle.  
  - Benchmark: produce a comparative report (exact vs fast_exact vs fast_approx) with latency, error rates, and quality proxies, attached to the job artifacts.  
  - Ensure high-volume traces are sampled and privacy-compliant (no token IDs; no raw text by default).

- [ADD v02.137] **Multi-session automation upgrades:** introduce guarded semi-autonomous loops over `MultiModelSession` (AutomationLevel.ASSISTED), with explicit budget caps, spawn limits, and FR-logged policy decisions.
- [ADD v02.137] **Workflow alignment:** represent `MultiModelSession` execution as (or within) a `workflow_run` that schedules child `model_run` jobs, so orchestration is auditable via the standard AI Job Model primitives.

- [ADD v02.186] KERNEL-004 cluster A â€” HBR enforcement at build+handoff gate (Â§5.6 + new CX-131 anchor), visual debugger product surface (Â§6.4 + Â§10.12), backend inspector plane (Â§6.5), non-hijacking GUI invariants (Â§6.6), swarm-agent harness (Â§6.7).
- [ADD v02.186] KERNEL-004 cluster B â€” SandboxAdapter trait + 3 adapters (Â§3.5) replacing ad-hoc sandbox notes (Â§5.2.5 cross-link).
- [ADD v02.186] KERNEL-004 cluster C â€” ModelRuntime + LocalModelAdapter (Â§4.6); 8 PRODUCTION inference techniques (Â§4.7.1) with Inference Lab UI (Â§10.14); MoD preliminary research only (Â§6.8).
- [ADD v02.186] KERNEL-004 cluster D â€” Memory V0+ self-improvement loop on fixed ~30-item HBR corpus (Â§4.8.3 + FEMS V1); distillation pipeline opt-in + content review (Â§4.8); ProcessOwnershipLedger Postgres table for ALL Handshake-spawned processes (Â§5.7); ModelManual surface (Â§10.15).

**Vertical slice**  
- Drop an audio file into Handshake.  
- Run transcription and see progress.  
- Open the resulting transcript document.  
- Inspect Job History and Flight Recorder entries for the ASR job and confirm model choice, status transitions, and any errors.
- [ADD v02.186] Run a Handshake validator session through Memory V0+ self-improvement loop on the fixed HBR corpus; observe sandbox-boxed model processes in ProcessOwnershipLedger; toggle 2 of the 8 inference techniques in Inference Lab UI; confirm visual debugger screenshots match WebView2 CDP capture.

9. **[ADD v02.52] Transcript retrieval compatibility (ACE-RAG-001)**
   - Define transcript selectors (`ts_range`, `segment_id`) as bounded selectors for `SourceRef` so reads are time-range bounded.
   - Store `source_hash` per transcript segment (and per derived chunk) so IndexDriftGuard can detect ASR regeneration drift.
   - ContextPack builder MUST support transcript targets and emit timestamped anchors (timecode + excerpt hint).
   - RAG Debugger MUST render transcript spans as time ranges and deep-link to the underlying media segment where available.

- [ADD v02.52] Ask a question over a transcript; RetrievalTrace shows timestamp spans, budgets, and (if applicable) drift flags when transcript is regenerated.


- [ADD v02.67] Extend Atelier Lens to time-based media:
  - Treat transcript spans and video time ranges as first-class `EvidenceRef.time_span` targets for role bundles.
  - Implement/validate at least one post-production role pipeline over long-form media (Editor or Color) using time-span evidence and producing typed deliverables.
- [ADD v02.79] Run auto-tag + quality scoring over a batch â†’ build a smart collection from tags/scores â†’ inspect diffs/drift via logs.


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**  
  A local worker role runs `fast_approx` under a waiver on a LayerSkip-capable model; `llm_exec_policy` shows approximate active + trace reference; system falls back to exact if unsupported.

**Key risks addressed in Phase 3**

- ASR pipeline is unreliable or too opaque.  
- Long-form capture produces transcripts that are hard to relate back to source media.  
- ASR jobs are not easily distinguishable or debuggable compared to other jobs.
- [ADD v02.138] CRM/memory privacy risk: contact-scoped memory used for mail drafting can leak PII or introduce unwanted bias. Mitigation: local-only defaults, classification + consent gating for cloud prompts, and DCC review of contact memory.
- Symbolic/Taste engines (SYM-001 + Taste Engine) accidentally redefine descriptor law or mutate descriptor rows instead of adding separate Derived overlays.
- NLP/media helpers could drift or silently alter base descriptors without overlays/provenance.
- MCP-based distillation/sampling flows accidentally run with write-capable tools or bypass the Gate/logging path, causing side effects or untraceable model changes.


- [ADD v02.52] ASR transcript regeneration changes evidence silently (drift); mitigated by per-segment source_hash + drift guard + explicit degraded-mode warnings.
- [ADD v02.131] Stage 3D patchset risk: applying canonicalization/optimization patches can mutate assets in ways that are hard to audit; mitigate via patchset artifacts, derived-asset-only outputs (no in-place mutation), and mandatory before/after hashes + diffable reports.


- [ADD v02.67] Time-based evidence is hard to audit and drifts when media is re-encoded; mitigated by time-span EvidenceRefs, per-segment/source hashes, and explicit drift flags in retrieval/debugger surfaces.
- [ADD v02.79] Stochastic drift and silent regressions â†’ require model/version pinning and artifacted outputs with comparison tools.


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Runtime fragmentation (policy requested but not observable).  
  - Quality regression without accountability.  
  - Trace overhead and privacy leakage.

- [ADD v02.186] Process orphaning across sandbox adapters; mitigated by ProcessOwnershipLedger HARD-write invariant + bounded async write perf path.
- [ADD v02.186] Inference-technique combinatorial explosion; mitigated by ExecPolicy-extended schema + per-adapter capability declarations + Work-Profile gating.
- [ADD v02.186] Memory self-improvement Goodhart collapse on HBR corpus; mitigated by 60/20/20 split + holdout-sentinel + multi-metric promotion floor.
- [ADD v02.186] MoD scope creep; mitigated by spec-deferring MoD to Â§6.8 research subsection only (no stub, no WP).

**Acceptance criteria**

- [ADD v02.131] Stage Studio: a 3D report + patchset can be reviewed in a Stage App; applying a patchset produces a new derived asset with before/after hashes and provenance; allow/deny and mutations are logged in Flight Recorder/Job History.


- At least one realistic audio file can be ingested, transcribed, and inspected end-to-end.  
- ASR jobs appear clearly in Job History and can be filtered and inspected separately.  
- Logs and debug views provide enough information to reason about ASR failures or poor transcripts.
- Scheduler/backpressure: queue depth/latency metrics visible; back-pressure behavior under load documented for ASR/media jobs.  
- Eval harness: a golden/eval suite runs and persists metrics; LLM-as-judge outputs (where used) recorded in Flight Recorder.  
- At least one distillation/promotion cycle uses the MCP sampling pipeline end-to-end (Student â†” Teacher), with sampling calls and eval metrics visible in Flight Recorder and tied to Skill Bank entries and checkpoints.
- Monaco AI actions respect capability scopes; AI-assisted code actions log provenance to Flight Recorder.
- Director/Composer/Atelier/Artist outputs attach to Canvas media nodes and embedded blocks with Flight Recorder provenance back to source plans/prompts/files.  
- Mail jobs: Job History/FR show mail job inputs/outputs (thread IDs, attachments used); draft-to-send flow blocked without `SEND_EMAIL`; confirmation logged; local-only policy enforced unless classification allows; pre-send checks + diffs/provenance recorded.
- [ADD v02.138] CRM/Contact memory: contact-scoped `MemoryItem`s can be created and reviewed; mail drafting may use them when scoped; cloud prompts omit high-sensitivity contact memory by default; DCC shows review + pack preview.
- NLP overlays/curation: Aligner/Lexicographer/Curator jobs recorded in FR with inputs/outputs; overlays attach by reference to sources; capability gates enforced; base descriptor/schema left unchanged.
- ASR media pre-processing: FFmpeg normalization parameters and audio stream selection recorded in FR; pre-processing failures expose logs.  
- Optional diarization: if enabled, diarization overlays/metadata appear with speaker labels and timestamps; base transcript stays unchanged; provenance recorded.  
- [ADD v02.36] ASR jobs are durable across restart (workflow state + artifacts remain inspectable).
- [ADD v02.36] A transcript can be regenerated from stored refs/config and compared (structure-level diff is acceptable).



- [ADD v02.44] A notebook/job run can be re-executed from stored inputs and recorded environment metadata; failures yield typed diagnostics and artifacts.
- [ADD v02.49] Demonstrate TTL + pinning on an ASR transcript + associated artifacts; expired unpinned artifacts are GCâ€™d and a `gc_report` artifact is emitted.
- [ADD v02.49] ASR Debug Bundle export uses canonical bundle hashing; same inputs/config produce a stable structural hash, and per-file manifests/bundle_index are recorded.

- [ADD v02.52] Transcript Q&A uses the same QueryPlan/Trace pipeline; evidence spans are time-bounded and replayable.


- [ADD v02.67] At least one long-form media artifact (audio/video) produces role bundles with `EvidenceRef.time_span` and clickable deep-links to the source segment.
- [ADD v02.67] A post-production role (Editor or Color) runs end-to-end on long-form media and emits typed deliverables with provenance; time-based Lens validators pass (including evidence presence and drift detection where applicable).
- [ADD v02.79] Re-running the same vision job under same model/version produces â€œwithin-policyâ€ stable structure; drift is detectable.


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Operator can run the same WP/MT twice (exact vs approximate with waiver) and see:
    - effective policy differences,
    - latency/throughput deltas,
    - explicit waiver linkage in telemetry.

- [ADD v02.186] HBR gate runs at gov-check and at every WP handoff; HBR_VIOLATION receipts appear in WP_RECEIPT stream.
- [ADD v02.186] All 8 PRODUCTION techniques toggleable from Inference Lab UI on a llama.cpp model (LoRA hot-swap, KV caching) and on a Candle model (Activation Steering, Refusal Vector, CAA, Self-Speculative Decoding, Subquadratic).
- [ADD v02.186] Abliteration runs as offline tool only; never observed in a generation hot path.
- [ADD v02.186] Memory V0+ loop demonstrates measurable validator PASS-rate lift on dev set without holdout regression.

**Explicitly OUT of scope**

- [ADD v02.131] Stage Studio full authoring (Spline-class), complex rigging/animation editors, and collaborative real-time 3D editing remain Phase 4.


- Real-time streaming captions.  
- Fine-tuning workflows for ASR models.  
- Complex diarization and speaker management UIs.
- [ADD v02.79] AI masks/segmentation, ComfyUI, HDR/pano/focus merges.


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Production-grade automatic policy selection.  
  - Cloud-provider dynamic depth controls beyond â€œreasoning strengthâ€ tags.

- [ADD v02.186] MoD implementation; EAGLE-3 (post-GA follow-on); subquadratic full feature parity beyond Phase-3 slice; KERNEL-005..008 reserved territory.

**Mechanical Track (Phase 3)**
- [ADD v02.131] Stage 3D mechanicals: implement `stage.3d.apply_patchset.v1` + scene-diff generation as mechanical jobs (artifact-first I/O); patchsets are artifacts; derived-asset outputs only; before/after hashes + provenance logged and reviewable.
- [ADD v02.136] **Programmatic tool calling (Code Mode)** (Â§6.0.2.8): provide a sandboxed â€œcode executionâ€ tool that can call other tools via the Tool Registry SDK, enabling deterministic loops/batching without round-tripping large tool outputs through the LLM context.

- Media engines: `Director` (FFmpeg/Manim) and `Composer` (LilyPond/Music21) producing DerivedContent with sidecar command/param/hash provenance.
- Composer MAY use **SoX** or **LMMS** for audio processing/rendering alongside LilyPond/Music21; provenance must include tool/version/params.  
- ASR jobs and media jobs MUST share the same Flight Recorder schema, capability gates, and Job History filtering.
- Acceptance: at least one ASR run and one media render each have full logs (model/tool, params, timings) and artifacts accessible from Job History.
- Include `Atelier` (creative planning) + `Artist` (image/raster/vector rendering) for creative assets; capability gating and provenance logging; plans and artifacts attach to Docs/Canvas.
- Add NLP/media helpers: `Aligner` (parallel text), `Lexicographer` (dictionary/thesaurus), and `Curator` (collections/playlists) with provenance and capability gates; wire their outputs as DerivedContent overlays.

- **Taste Engine + SYM-001 as descriptor consumers (not law changes)**  
  - Wire the Taste Engine and SYM-001 so that they operate strictly as consumers of existing `DescriptorRow` and `TextDescriptorRow` data from Phase 2.  
  - Any new fields (layer scores, motif activations, taste scores) are stored as **DerivedContent overlays**, not as edits to the underlying descriptor rows.  
  - Phase 3 MUST NOT change DES-001 / IMG-001 / TXT-001 schema or invariants; any evolution of descriptor law happens in a future, explicit schema-migration phase.  
  - Acceptance: at least one Taste Engine / SYM-001 job can be inspected in Flight Recorder showing:
    - Inputs: descriptor IDs only (no direct RawContent edits).  
    - Outputs: symbolic/taste artifacts linked by reference to descriptors.  
    - No mutations to descriptor rows in the Corpus.
- [ADD v02.36] ASR job profile records model name/version + diarization/segmentation config + artifact hashes/handles to support reproducibility and comparisons.



- [ADD v02.44] Notebook execution as Jobs: `science.jobs.notebook_engine` (Jupyter-backed) producing typed outputs + artifacts and capturing failures as diagnostics.
- [ADD v02.44] Reproducibility bundles: `science.repro.run_bundles` (ReproZip capture) tied to notebook/script runs with stored bundle artifacts.
- [ADD v02.67] Ensure ASR/Media jobs emit time-span EvidenceRefs compatible with Atelier Lens and that Lens jobs can consume ASR transcript selectors and frame extracts without unbounded reads.


- [ADD v02.68] Conformance for media engines: Director/Composer runs must satisfy v1.2 conformance (artifact-only I/O, budget caps, provenance completeness, determinism class declared). D2 outputs MUST include canonicalization rules for stable structural hashes.
- [ADD v02.79] Implement `engine.vision` wrapper as governed job; proxy-first inputs by default.

- [ADD v02.115] **AI-Ready Data Architecture (Â§2.3.14) - Phase 3:**
  - Implement event-driven index updates (<5s latency for content changes)
  - Implement semantic chunking for prose/notes (topic boundary detection via embedding similarity)
  - Implement multi-granularity indexing (paragraph, section, document levels searchable)
  - **[REMEDIATION]** Wire Skill Bank training data selection from retrieval quality signals (Â§2.3.14.9) - extends existing Skill Bank schemas with data_trust_score and retrieval feedback
  - Implement `data_quality_audit_v1` mechanical job for periodic health checks
  - Implement `antipattern_detector_v1` for automated anti-pattern detection and remediation suggestions
  - Acceptance: index updates propagate within 5s; multi-granularity retrieval visible in debugger; Skill Bank receives retrieval quality signals

- [ADD v02.116] **Locus Work Tracking System (Â§2.3.15) - Phase 3:**
  - Implement PostgreSQL backend with workspace multi-tenancy (Â§2.3.15.5, Â§2.3.15.8)
  - Implement CRDT operation-based conflict resolution with vector clocks (Â§2.3.15.6)
  - Implement WebSocket real-time collaboration (PostgreSQL LISTEN/NOTIFY + WebSocket broadcast) (Â§2.3.15.8)
  - Implement workspace member roles and permissions (owner, admin, member, viewer)
  - Implement cross-workspace WP references (e.g., vendor WPs in shared workspace)
  - Implement WP auto-archival policy (configurable days-until-archive, status-based rules)
  - Scale testing: 100K WPs per workspace, 100 concurrent users, sub-500ms query latency (Â§2.3.15.9)
  - Acceptance: multi-user collaboration verified; CRDT conflicts resolve deterministically; WebSocket updates propagate <1s; performance targets met at scale


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Runtime adapter + event emission + trace artifact pipeline.  
  - Benchmark harness + replay tools.

**Atelier Track (Phase 3)**
- [ADD v02.67] Add explicit dual-contract examples and fixtures for post-production roles over time-based media (Editor/Color/VFX), including deliverable kinds and evidence requirements.
- [ADD v02.67] Extend Role Glance/Claim heuristics to time-based inputs (transcripts, keyframes, shot boundaries) while keeping the claim pass cheap and budgeted.
- [ADD v02.79] Tags panel + smart collections UI fed from vision outputs.


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Minimal UI to surface effective policy and downgrades in Job History.

**Distillation Track (Phase 3)**
- Implement student runs, checkpoint creation, and eval/promotion gates vs teacher and previous checkpoints.
- Enforce rollback via checkpoint lineage; gate on security flags and collapse indicators; persist promotion decisions in Flight Recorder.
- Use the MCP sampling pipeline (Section 11.3.5) for student/teacher eval runs where applicable:
  - Distillation/eval calls use `sampling/createMessage` via the MCP Gate.
  - All such calls are logged into Flight Recorder and `fr_distillation_samples` with clear linkage to Skill Bank entries and checkpoints.
- Acceptance: at least one promotion cycle logged end-to-end with metrics, lineage, and gate outcomes; rollback tested.
- [ADD v02.157] Any local-model promotion or adapter merge MUST prove teacher/student/context-pack lineage, benchmark deltas, and rollback-safe checkpoint compatibility before promotion decisions become effective.



- [ADD v02.115] **[REMEDIATION] MT Executor LoRA training automation:** Automated LoRA fine-tuning triggered from DistillationCandidate accumulation; implement training job profile, checkpoint management, and A/B comparison against previous LoRA versions. (Extends existing Skill Bank distillation infrastructure.)
- [ADD v02.115] **MT failure category refinement:** Refine FailureCategory taxonomy based on accumulated escalation data; add new categories as patterns emerge; update LoRA training targets accordingly.
- [ADD v02.115] Acceptance: LoRA training jobs visible in Job History; checkpoints stored with provenance; regression tests prevent LoRA degradation.
- [ADD v02.79] Promote successful â€œculling + taggingâ€ flows into reusable workflows/presets.


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Optional: learn a safe default mapping from `speed_preset` â†’ exec_policy per runtime.

### 7.6.6 Phase 4 â€” Collaboration & Extension Ecosystem

**Goal**  
Move from a single-user tool to a collaborative, extensible platform, while preserving and extending observability and debug tools.

- [ADD v02.105] Phase coverage is governed by Â§7.6.1 Coverage Matrix; Phase 0 is closed and MUST NOT be used for scheduling newly discovered requirements (remediate in Phase 1 or later).
- [ADD v02.52] Make ACE-RAG-001 collaboration-safe: per-user capability-gated catalogs/routes, pack regeneration policy across devices, and audit-preserving cache behavior.
- **[ADD v02.130] Make Loom collaboration-safe**  
  LoomBlocks + LoomEdges must support multi-user workspaces: CRDT-safe sync, capability-gated edge/tag creation, and audit-friendly event logging for shared libraries.
- **[ADD v02.131] Make Stage collaboration-safe + extension-ready**  
  Stage sessions, Stage Apps, and Stage Bridge must work in multi-user workspaces: per-user capability grants, audit logs, and hash-pinned Stage App bundles; Stage Studio evolves toward Spline-class authoring as an optional surface.



- [ADD v02.68] Treat engine adapters as installable/pinnable extension artifacts: registry updates are audited, adapters are hash/version pinned, and no engine becomes callable without passing conformance.

- [ADD v02.79] Make Photo Studio collaboration-safe (recipes/collections/presets sync without breaking provenance or determinism guarantees).


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**  
  Turn dynamic compute into a safe, optional ecosystem capability: multiple runtimes can support `exec_policy`, operators can compare policies, and governance remains uncompromised.

**MUST deliver**

1. **Collaboration & sync**  
   - Integrate a CRDT library (e.g. Yjs) with the existing document and canvas model, as described in Section 7.3.  
   - Define and implement a sync topology (file-based or server) with a **`y-websocket` provider** for servered sync; support file-based sync for distributed/offline collaboration.  
   - Ensure Workflow Engine, AI Jobs, and Flight Recorder behave correctly under concurrent edits:
   - Provide a **CRDT Time Machine / Merge Visualizer** in Operator Consoles:
     - Replay concurrent edit sequences and inspect merge outcomes.
     - Deep-link merge/conflict events to the underlying CRDT ops and affected entities.

     - Conflicts are visible and traceable.  
     - Job history clearly shows which user and which device triggered which actions.

2. **Multi-user semantics**  
   - Introduce an authentication/session model.  
   - Define how AI jobs behave when multiple users interact with the same artefacts (ownership, consent, capability scope, per-user audit trails).  
   - Extend debug tooling to:
     - Filter Flight Recorder and Job History by user, workspace, and device.  
     - Inspect collaborative sessions and their timelines.

   - Bind MCP sessions to user identity and workspace context:
     - MCP client connections from the coordinator carry WSID and user/session identifiers.  
     - The MCP Gate enforces per-user capability scopes and logs which user triggered each MCP tool call.  

3. **Plugin / extension system (initial)**  
   - Design an internal plugin API built on top of the AI Job Model and capability system.  
   - [ADD v02.131] Treat Stage Apps as first-class internal plugins: Stage Bridge API is the UI-facing RPC layer; Stage App bundles are hash/version pinned and capability-scoped; Stage Host origin isolation is enforced and all privileged actions are logged to Flight Recorder.
   - Expose safe extension points:
     - New workflow nodes.  
     - New AI Job profiles or capability profiles.  
   - Require that plugins:
     - Use the same logging, metrics, and Flight Recorder frameworks.  
     - Register their actions so they appear in Job History and traces.  
   - Prepare for external plugins by aligning with security and sandboxing constraints defined in Section 5.2, and by favouring MCP-based tool servers as the primary extension mechanism (plugins as MCP servers behind the Gate).  
   - Implement an initial sandbox for untrusted plugins using **WASM** (and optionally **Pyodide** for Python), enforcing the capability model (default-deny) and logging all calls via Flight Recorder.  

4. **Security and privacy hygiene**  
   - Document how logs, Flight Recorder data, and debug traces handle sensitive content.  
   - Provide at least basic controls for:
     - Clearing or rotating logs.  
     - Exporting/importing data safely.  
5. **Mail advanced governance/analytics/taste**  
   - Classification ladder + tags (PUBLIC â€¦ HIGHLY_RESTRICTED) and routing rules for cloud/local engines and connectors.  
   - Chronicle/Analyst dashboards for mail analytics; Polyglot/Red Pen/Sentiment/Anonymizer wired into mail flows.  
   - Taste models for mail reply style per client/classification.
6. **Spatial/optional mechanical domains (gated/optional)**  
   - Treat `Cartographer`/`Navigator`/`Geo` as optional extensions gated by network/device capabilities.  
   - Treat `Decompiler`/`Homestead`/`Sous Chef` as plugin-scope only, enabled explicitly with capability grants and FR logging.  
   - Keep heavy engines (`Spatial`/`Machinist`/`Simulation`/`Hardware`/`Guide`) tied to device/network/GPU grants and reproducibility checks.



7. **[ADD v02.36] Plugin capability precedence + bypass hardening (enforced + tested)**  
   - Enforce precedence resolution (**plugin > workspace > builtin**) deterministically and log resolutions for auditability.  
   - Plugins MUST NOT bypass Gate/Workflow/Flight Recorder; all actions/tool calls route through the same capability checks and trace plumbing.

8. **[ADD v02.36] Mail/Calendar offline-first mode (optional if enabled) promoted to MUST**  
   - When enabled, operate offline-first with incremental sync, attachment scanning, and retention/consent defaults; sync operations are traceable.

   

9. **[ADD v02.44] Extension substrate: WASM plugins (capability-scoped)**  
   - Implement `tech.plugins.wasm_runtime` using Extism + Wasmtime as the default sandbox for optional domain modules.


10. **[ADD v02.47] Collaborative Charts & Decks + extension templates (CRDT-aware)**  
   - CRDT-safe multi-user editing for chart specs and deck slide layouts; per-user attribution visible in Job History/Flight Recorder.  
   - Extension-delivered chart templates, dashboard layouts, and deck themes (capability-scoped; no bypass of governance).  
   - Plugin-provided exporters/renderers MUST register as mechanical jobs (engine/version/params/hashes/policy logged; no bypass).


11. **[ADD v02.79] Photo Studio collaboration + extension hooks**  
   - [ADD v02.79] Conflict strategy for recipe edits + collection membership + preset updates.  
   - [ADD v02.79] Extension points: importers/export profiles/presets as plugins (gated; no-bypass).  

- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem) â€” Phase 4 or later**
  - Runtime compatibility: at least two runtimes/providers support `exec_policy` with deterministic downgrade + `llm_exec_policy` emission.  
  - Policy registry: named, versioned exec_policy presets (`policy_id`) that can be referenced in Work Profiles and jobs (without forcing schema rewrites).  
  - Cross-role/dynamic-role support: inheritance model for dynamic roles + per-role compute overrides (see Â§4.5.6.2).  
  - Operator-grade reporting: dashboards for requested vs effective policy, approximate usage frequency, and waiver compliance audits.  
  - Export tooling: trace artifacts and summaries exportable with privacy controls and retention enforcement.



12. **Loom collaboration + shared libraries** [ADD v02.130]  
   - Extend sync/collaboration layer to include LoomBlocks and LoomEdges (UUID-stable, CRDT-safe). Concurrent edits to block metadata, tags, and mentions must converge deterministically.
   - Enforce capability-gated mutations for shared workspaces: edge creation, tag creation, pinning, and deletion must respect per-user permissions and produce auditable Flight Recorder trails (Â§11.5.12).
   - Support shared tag hubs (`TAG_HUB` blocks) with sub-tag hierarchies and conflict-safe merges.
   - Scale Loom view/search for shared workspaces (Tierâ€‘2 Postgres full-text as needed) while preserving local-first behavior when offline.

- [ADD v02.137] **Cross-workspace and multi-operator routing:** allow sessions to be routed across multiple workspaces and shared between multiple human operators with explicit consent boundaries.
- [ADD v02.137] **Third-party session isolation:** hard multi-tenant isolation for third-party â€œappsâ€ that spawn sessions, including per-app quotas, provenance, and revocation.
- [ADD v02.137] **Advanced recovery and audit:** deterministic replay of session spans and tool calls for high-stakes audits (within policy), using content-hash + artifact refs.

**Vertical slice**  
- Two users (or two devices) edit the same document using the chosen sync topology.  
- One user triggers an AI action that modifies the shared document.  
- Both users see the changes.  
- Job History and Flight Recorder show which user triggered the job, how it ran, and how it interacted with sync/CRDT.
- [ADD v02.47] Two users co-edit a dashboard + deck â†’ run export â†’ verify attribution, capability grants, and export policy in Flight Recorder/Job History.

11. **[ADD v02.52] Multi-user ACE-RAG governance (packs, caches, traces)**
   - Capability-gate Semantic Catalog and routing hints per user/workspace; catalogs MUST NOT reveal selectors/paths outside granted scope.
   - Define cross-device policy for ContextPacks:
     - ownership/attribution,
     - regeneration triggers,
     - stale/invalid handling under concurrent edits.
   - Define multi-user cache semantics:
     - cache keys include policy_id + user scope,
     - caches never leak evidence across users without explicit share grants,
     - replay mode preserves trace integrity across devices.
   - RAG Debugger and Index Doctor MUST show user/device attribution for QueryPlan/Trace and for any pack/caching decisions.

- [ADD v02.52] Two collaborators ask the same question on shared content; traces show per-user policy/capability gating and no evidence leakage across users/devices.


- [ADD v02.67] Package Atelier roles/contracts as extensions:
  - Define a â€œRole Packâ€ format containing `AtelierRoleSpec`, schemas, validators, and deliverable templates.
  - Enforce capability gating + provenance: extension-provided roles MUST run through Workflow Engine + Flight Recorder and must not bypass policy/export rules.
- [ADD v02.79] Two clients edit the same photo recipe and resolve conflict â†’ export reproducibly with full provenance.


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**  
  Operator switches between two runtimes supporting exec_policy, compares fast_exact vs standard on a WP/MT, and exports a policy + trace report suitable for audit.


- **Loom collaboration loop** [ADD v02.130]
  - Two users open the same shared workspace Loom library.
  - User A imports a file into Loom; User B sees it appear in All view after sync.
  - Both users add tags/mentions to the same block; conflicts converge without duplicate edges.
  - Audit trail: Flight Recorder shows who created which edges/tags and when.
  - Permissions check: a user lacking tag-create capability cannot create new tag hubs (but can use existing tags, if allowed).

**Key risks addressed in Phase 4**
- [ADD v02.130] Loom collab risks: CRDT convergence for edge graphs, duplicate edge prevention under concurrency, and permission/audit correctness in shared tag hubs.


- [ADD v02.47] Multi-user attribution ambiguity for charts/decks (who changed what, and what exactly was exported) if lineage is not captured.
- [ADD v02.47] Malicious/unstable render/export extensions compromise determinism or leak data unless sandboxed and capability-gated.

- [ADD v02.49] Multi-user artifact drift (different bytes across devices) breaks shared provenance; mitigate via SHA-256 dedupe + canonical bundle hashing + explicit ExportRecords across collaborators.

- Collaboration behaviour is inconsistent or not auditable.  
- Plugins bypass governance, capabilities, or observability.  
- Logs and debug tools become unusable in multi-user scenarios.
- Optional/spatial/plugin engines leak data or bypass safety if not explicitly gated and logged.
- MCP-based plugins or external MCP servers bypass the Gate or user/session binding, making actions untraceable or undermining per-user capability scopes.


- [ADD v02.44] Plugin capability model gaps: any bypass of Gate/Workflow/Flight Recorder breaks auditability and safety; the plugin runtime MUST remain deny-by-default.
- [ADD v02.44] Hardware/network access risk for CNC daemon connections and simulation tooling; capability scoping and logging must be complete.
- [ADD v02.67] Role Packs/extensions can bypass Lens validators or mutate Raw/Derived unless the same gates apply; mitigated by default-deny capabilities, mandatory validator packs, and export/write-back prohibition enforced by the host runtime.
- [ADD v02.79] Sync conflicts and provenance corruption â†’ sync only versioned schemas + artifact refs; forbid raw mutation paths.


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**
  - Operator confusion (what actually ran).  
  - Governance drift (approximate becoming implicit).  
  - Ecosystem incompatibility across runtimes.

**Acceptance criteria**
- [ADD v02.130] Loom collaboration: LoomBlocks/edges sync across users, converge without duplicate edges, respect capability gating, and generate auditable FR-EVT-LOOM trails.


- Collaborative edits are correctly synced and traceable in logs.  
- AI jobs in collaborative sessions are correctly attributed to users/devices.  
- Plugins can register actions and appear in Job History without bypassing capabilities or Flight Recorder.  
- At least one MCP-based extension (plugin or external server) is integrated; its MCP tool calls are visible in Job History and Flight Recorder with correct user/session attribution and capability metadata.
- Minimal security/privacy documentation exists for logs and debug data.
- Conflict handling UX present; conflicts are surfaced and traceable.  
- User/device filters available in Flight Recorder and Job History.  
- Plugin capability precedence (plugin > workspace > builtin) enforced; capability cache expiry honored.  
- Mail/Calendar (optional if enabled) operate offline-first with capability-gated sync, attachment scanning, and retention/consent defaults.  
- Heavy/hardware engines enforce device/network/GPU safety gates; commands/artifacts are reproducible and denial paths tested.
- Spatial/Machinist/Simulation/Hardware/Guide outputs link back to Canvas/Docs/Monaco artifacts with provenance; commands/params/artifact hashes recorded for reproducibility.
- Mail advanced: classification tags persist on MailMessage + descriptors; routing policies enforced (cloud vs local); FR logs classification decisions; analytics dashboards populated; taste model outputs logged with provenance.
- Optional/plugin engines (Cartographer/Navigator/Geo/Decompiler/Homestead/Sous Chef) only enabled via explicit plugin/extension switches with capability grants; FR logs inputs/outputs and provenance for any run.
- At least one sandboxed plugin (WASM or Pyodide) runs with default-deny capabilities, registers actions in Job History, and logs calls/events to Flight Recorder; attempts to bypass capabilities are denied and logged.  



- [ADD v02.44] A plugin-provided vertical slice runs end-to-end with explicit capability grants, full Flight Recorder provenance, deterministic artifact outputs, and (where applicable) multi-user attribution.
- [ADD v02.67] At least one Role Pack extension installs cleanly, registers roles/contracts, and runs Lens jobs under capability gates with full Flight Recorder provenance and validator enforcement.
- [ADD v02.67] Cross-user attribution for Lens runs is visible in Job History/Flight Recorder where collaboration is enabled; extension provenance includes pack id/version/hash.
- [ADD v02.79] Concurrent edits converge; all exports remain attributable to specific recipe versions + engine versions.


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**
  - A job can be replayed (or deterministically explained as non-replayable) with clear â€œeffective policyâ€ disclosure.  
  - Auditors can list all approximate executions over a time range with waiver refs and affected WPs/MTs.

**Explicitly OUT of scope**
- [ADD v02.130] Loom: "block-as-app" programmable views and cross-workspace public Loom libraries remain future-surface work (see Â§10.12 roadmap).


- Phase 4 does not expand core single-user UX beyond what is required for collaboration.
- Complex plugin marketplaces, monetization, and third-party billing.
- Unbounded external write-back/sync targets without explicit capability grants and provenance.
- Automatic sharing of RawContent/DerivedContent across collaborators without explicit user consent controls.
- [ADD v02.79] Full Affinity/Illustrator parity; advanced compositor; full marketplace.


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**
  - Mandatory layer-wise inference for all users.  
  - Auto-enabling approximation based on heuristics without operator consent.

**Mechanical Track (Phase 4)**
- [ADD v02.130] Loom shared-workspace query scaling: optional Postgres Tierâ€‘2 indexes and background rebuild jobs for metrics/backlinks at scale.


- [ADD v02.47] Extension-provided chart renderers/exporters (PPTX/PDF/HTML) MUST route through Workflow Engine + mechanical runner + Flight Recorder with full provenance (engine/version/params/hashes/policy).
- Heavy/hardware engines: `Spatial` (CAD), `Machinist` (CAM/G-code), `Simulation` (FEA/CFD), `Hardware` (camera/USB/serial), `Guide` (routing/live checks) behind explicit device/network/GPU grants and safety gates.
- Spatial/extensions: `Cartographer` (maps/tiles), `Navigator` (routing), and `Geo` (GIS queries) gated by network/device capabilities; treat as optional plugin-scope if not core to a release.
- Advanced/optional domains: `Decompiler` (reverse engineering), `Homestead` (home logistics), and `Sous Chef` (culinary) only under explicit plugin/extension enablement with capability gates and FR logging.
- Plugins must register mechanical actions through Workflow Engine + Flight Recorder; no bypass of capabilities or logging.
- Acceptance: CAD -> CAM -> Simulation vertical slice with safety validation and reproducible outputs; hardware connector exercised with a mock device; spatial engines show provenance and safety gating; optional/plugin engines logged with explicit capability grants; multi-user attribution visible in Job History/Flight Recorder.


- [ADD v02.44] Woodworking/Digital fabrication reference extension vertical:
  - `wood.primitives.shopgrade`
  - `wood.jobs.nesting_2d` (Deepnest `product_managed_process`)
  - `wood.jobs.job_packet_compiler` (qpdf + libvips)
  - `wood.validators.toolpath_simulation` (CAMotics `product_managed_process`)
  - `wood.connector.machine_daemon` (CNCjs `operator_configured_adapter`)
- [ADD v02.44] Creative interoperability modules:
  - `creative.interop.timeline_otio` (OTIO import/export)
  - `creative.review.annotations` (Annotorious) â€” only after pinned-version license verification is recorded in the OSS Component Register.
- [ADD v02.67] Add Role Pack install/uninstall, version pinning, and deny-by-default capabilities for extension-provided Lens jobs.


- [ADD v02.68] Engine adapter packaging posture: adapters/extensions MUST be version+hash pinned, registered (registry is authoritative), and conformance-gated. High-risk capabilities (device/network/secrets/GPU) require explicit per-user grants with full provenance.
- [ADD v02.68] Add `Sovereign` engine slice for collaboration: cryptographic signing/verification and key-handling flows run as mechanical jobs under secrets-use policy and are fully auditable (inputs/outputs as artifacts; no raw key material in logs).
- [ADD v02.79] Plugin packaging + conformance requirements for any Photo engine adapter before it becomes callable.

- [ADD v02.115] **AI-Ready Data Architecture (Â§2.3.14) - Phase 4:**
  - Implement distributed index sharding for large workspaces (>100k documents)
  - Implement advanced cross-modal retrieval (find code by describing its visual output, find images by code that generated them)
  - Implement real-time quality monitoring dashboards in Operator Consoles
  - Implement collaborative index synchronization (multi-user index consistency)
  - Implement embedding model migration automation (batch re-embedding when models upgrade)
  - Acceptance: index sharding transparent to queries; cross-modal queries work across Codeâ†’Image, Imageâ†’Code paths

- [ADD v02.116] **Locus Work Tracking System (Â§2.3.15) - Phase 4:**
  - Implement advanced analytics: WP completion velocity trends, MT success rate by model/LoRA, blocking dependency hot-path analysis
  - Implement AI-powered insights: WP priority recommendations based on dependency impact, MT escalation pattern detection, auto-suggest decomposition for large WPs
  - Implement cross-workspace WP aggregation queries (e.g., all vendor-related WPs across all projects)
  - Implement WP templating system: reusable WP templates with auto-fill task packet structures
  - Implement multi-user Locus governance: per-user WP creation quotas, collaborative gate approval workflows, attribution tracking
  - Implement Locus plugin API: allow extensions to add custom WP fields, dependency types, and query filters
  - Acceptance: AI recommendations visible in UI; cross-workspace queries performant; WP templates reduce creation time; plugin extensions registered and capability-gated


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**
  - Policy registry + schema validation + cross-runtime adapters.  
  - Audit/reporting pipelines.

**Atelier Track (Phase 4)**
- [ADD v02.67] Role Packs: publish/install flows for `AtelierRoleSpec` + schemas + validators + templates; ensure pack hashes are recorded and referenced by Lens outputs.
- [ADD v02.67] Multi-user sharing: allow sharing Deliverable Bundles (not Raw/Derived corpora) under explicit grants; ensure no-censor remains internal while export projections are policy-gated.
- [ADD v02.79] Collaborative review UI for recipe diffs + preset sharing.


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**
  - UX for policy selection, waiver management, and effective-policy inspection.

**Distillation Track (Phase 4)**

- [ADD v02.47] Multi-user export governance for charts/decks: per-user consent and export policy selection recorded with lineage across collaborators/devices.
- Multi-user governance for Skill Bank artifacts: per-user attribution, consent, and export controls; plugin/extensibility hooks use the same logging/capability model.
- Support secure sharing/off-device export only via explicit capability grants; maintain lineage across collaborators/devices.
- Acceptance: distillation artifacts respect multi-user capability scopes; Job History/Flight Recorder show user/device attribution for distillation jobs and exports.

- [ADD v02.115] **MT Executor parallel wave execution:** Implement concurrent MT execution within dependency-safe waves; add wave scheduling, resource coordination, and progress aggregation for parallel MTs.
- [ADD v02.115] **MT cloud escalation governance:** Implement cloud_escalation_allowed policy with per-user consent, cost tracking, and capability gates for cloud model usage in escalation chains.
- [ADD v02.115] Acceptance: parallel wave execution demonstrated with >2 concurrent MTs; cloud escalation requires explicit capability grant and logs cost/usage.

- [ADD v02.115] **AI-Ready Data Architecture â†’ Multi-user LoRA governance (Â§2.3.14.9):**
  - Implement per-user training data attribution and consent controls
  - Implement shared vs private LoRA model governance (who can use locally-trained models)
  - Implement training data export controls (prevent sensitive data leakage via LoRA sharing)
  - Implement cross-device LoRA synchronization with provenance tracking
  - Acceptance: LoRA training data respects per-user consent; shared LoRAs carry full provenance; export denies sensitive training sources
- [ADD v02.79] Shared â€œhouse styleâ€ presets and taste descriptors derived from accepted edits (opt-in).


**Key Takeaways**  
- The roadmap is **architecture-aligned and debug-first**: every phase explicitly requires health checks, structured logging, Flight Recorder integration, and at least one human-usable debug surface.  
- **Vertical slices** ensure each phase ends with a real, end-to-end scenario you can manually test, not just abstract infra.  
- Phases are used to **burn down risk**: stack stability in Phase 0, AI Jobs + workflows + observability in Phase 1, ingestion/RAG in Phase 2, ASR in Phase 3, and collaboration/plugins in Phase 4.  
- Cross-cutting concernsâ€”migrations, security/privacy of logs, dev experience, ADRsâ€”are included so they are not forgotten while focusing on features.
