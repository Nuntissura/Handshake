## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Editor-Hardening-v2
- CREATED_AT: 2026-01-16T22:07:46.9984801Z
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.112.md
- SPEC_TARGET_SHA1: 33b50fe7d70381c3eb2a53871f673e1d442633e1
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja160120262314
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Editor-Hardening-v2

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Editor integration drift risk: Tiptap/BlockNote and Excalidraw are intended to be projections over the same workspace model. Any UI path that mutates content while bypassing the shared job/capability/context plumbing creates a "shadow pipeline" and breaks cross-view determinism (spec anchor 2.2.0).
- No Silent Edits enforcement risk: the backend StorageGuard requires job/workflow context for AI-authored writes. If editor-triggered AI flows write via endpoints that do not supply the required context, writes will be rejected (HSK-403-SILENT-EDIT) or, worse, accepted without proper MutationMetadata (spec anchor 2.9.3).
- UI conformance gap (expected audit focus): ensure editor-triggered writes (doc blocks and canvas nodes/edges) always persist MutationMetadata fields and do not introduce tool-specific storage schemas or per-tool write paths.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE (this refinement scopes editor wiring + persistence traceability; it does not introduce new Flight Recorder event IDs).

### RED_TEAM_ADVISORY (security failure modes)
- Silent edits undermine auditability: AI-authored mutations without job/workflow IDs become untraceable, weakening provenance guarantees and making incident response unreliable.
- Shadow pipelines: tool-specific write paths can bypass capability gating and policy checks, enabling unauthorized side effects or data exposure through inconsistent enforcement.
- Partial fixes: UI-only or backend-only changes that do not preserve end-to-end context propagation can create inconsistent behavior across tools and regress determinism.

### PRIMITIVES (traits/structs/enums)
- Backend (existing per spec target): MutationMetadata, WriteActor, StorageGuard, HSK-403-SILENT-EDIT error.
- Frontend (expected): consistent write-context propagation when invoking backend persistence from editor surfaces (document + canvas).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.112 defines tool integration invariants (2.2.0) and a normative mutation traceability + no-silent-edits guard contract (2.9.3). Editor hardening is an implementation conformance task that ensures Tiptap/Excalidraw do not violate these invariants.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec already specifies the normative tool-integration and mutation-traceability requirements; this WP is a repo conformance remediation, not a spec gap.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 2.2.0 (Tool Integration Principles)
- CONTEXT_START_LINE: 1092
- CONTEXT_END_LINE: 1129
- CONTEXT_TOKEN: ### 2.2.0 Tool Integration Principles
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.2.0 Tool Integration Principles

  Handshake intentionally avoids separate "doc mode", "canvas mode", or "sheet mode" at the data level. All tools and views operate over the same workspace model:

  - **Entities:** Workspace, Project, Document, Block, Canvas, Canvas Node, Table, Task/Event, Asset, External Resource, Workflow/Automation (Section 2.2.1).
  - **Layers:** RawContent, DerivedContent, DisplayContent with their rules (Section 2.2.2).
  - **Graph:** Knowledge graph and Shadow Workspace indexing (Sections 2.3.7\u00e2\u20ac\u201c2.3.8).
  - **Jobs:** AI Job Model and artefact-specific profiles (Sections 2.5.10 and 2.6.6).

  Principles:

  1. **Single workspace graph.**
     - Mechanical integrations (Docling, ASR, converters, image tools) **MUST** read/write workspace entities via the same Raw/Derived/Display model and IDs as the UI.
     - UI components (docs, canvases, tables) are different **projections** of this graph, not separate stores.

  2. **Tool-agnostic core schema.**
     - The unified node schema (Section 2.2.1.1) is the primary contract.
     - Tools and views **MAY** attach extra metadata, but **MUST NOT** require tool-specific storage schemas for core behaviours.

  3. **Jobs, not modes.**
     - All non-trivial operations (import, transforms, ASR, bulk edits) **SHOULD** run as AI Jobs (Section 2.6.6) or workflow nodes, regardless of which tool initiated them.
     - The system treats these as typed operations in the workflow engine, not as opaque per-tool pipelines.

  4. **Mechanical tools as first-class citizens.**
     - Docling, ASR engines, OCR, converters, and similar subsystems are treated as **mechanical tools** behind the Model Runtime Layer (Section 2.1.3).
     - Their outputs **MUST** land as RawContent/DerivedContent for workspace entities so that all downstream tools can consume them.

  5. **Cross-view reuse by default.**
     - Content imported or produced in one view (e.g. Docling-imported table, ASR transcript, Docling-derived figure captions) **SHOULD** be accessible in others without copy-paste:
       - Doc blocks appear as canvas cards.
       - Tables participate in docs and dashboards.
       - Transcripts and extracted tables are indexed by the Shadow Workspace and available to all agents.

  6. **Explicit capability boundaries.**
     - Tools, including OSS components, operate through the capability and policy system (Section 5.2, AI Job Model).
     - There is no privileged "Excel-only" or "Word-only" engine; everything uses the same capability-scoped operations.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 2.9.3 (Mutation Traceability (normative) + Storage Guard)
- CONTEXT_START_LINE: 9249
- CONTEXT_END_LINE: 9319
- CONTEXT_TOKEN: ### 2.9.3 Mutation Traceability (normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.9.3 Mutation Traceability (normative)

  To satisfy the traceability invariant (\u00c2\u00a77.6.3.8), every mutation to `RawContent` (e.g., document blocks) MUST persist metadata identifying the source of the change.

  1. **Storage Requirement:** Database tables for `blocks`, `cells`, and `nodes` MUST include `last_actor`, `last_job_id`, and `last_workflow_id` columns.
  2. **Audit Invariant:** Any row where `last_actor == 'AI'` MUST have a non-null `last_job_id` referencing a valid AI Job.
  3. **Silent Edit Block:** The storage guard (\u00c2\u00a7WP-1-Global-Silent-Edit-Guard) MUST verify that `MutationMetadata` is present and valid for all AI-authored writes.

  #### 2.9.3.2 Storage Guard Trait

  The application MUST implement the `StorageGuard` trait for all persistence operations. This trait acts as the final gate against silent edits.

  **Guard Implementation Requirements:**

  1.  **AI Write Context:** If `actor == WriteActor::Ai`, the guard MUST fail if `job_id` is `None`.
  2.  **Traceability Anchor:** The guard MUST generate a unique `edit_event_id` (UUID) for every successful validation and return it in `MutationMetadata`.
  3.  **Error Codes:** Use `HSK-403-SILENT-EDIT` for rejection.

  **Integration Invariant:**
  All database persistence methods in the `Database` trait (e.g., `save_blocks`, `update_canvas`) MUST call `validate_write` and persist the returned `MutationMetadata` fields.
  ```
