# Handshake Project Task Board (Phase 1: EXHAUSTIVE STRATEGIC AUDIT)

## Spec Authority Rule [CX-598] (HARD INVARIANT)

**The Roadmap (Section 7.6) is ONLY a pointer. The Master Spec Main Body (Sections 1-6, 9-11) is the SOLE definition of "Done."**

| Principle | Enforcement |
|-----------|-------------|
| **Roadmap = Pointer** | Section 7.6.x items point to Main Body sections where requirements are defined |
| **Main Body = Truth** | Every MUST/SHOULD in Sections 1-6, 9-11 must be implemented - no exceptions |
| **No Debt** | Skipping requirements poisons the project; later phases inherit rotten foundations |
| **No Phase Closes** | Until ALL referenced Main Body requirements are VALIDATED |

**Why:** Handshake is complex software. Treating roadmap bullets as requirements (instead of pointers) leads to surface-level compliance, technical debt, and project failure.

This board provides an exhaustive tracking of all Roadmap items from A7.6.3. Phase 1 cannot close until every item below is validated against Master Spec v02.99.

---


## Ready for Dev
- **[WP-1-Operator-Consoles-v2]** / FAIL (revalidation): `just post-work WP-1-Operator-Consoles-v2` fails (C701-G05 post_sha1 mismatch, C701-G04 window drift) for `src/backend/handshake_core/src/flight_recorder/duckdb.rs`. [READY FOR DEV]
- **[WP-1-MEX-v1.2-Runtime-v2]** / FAIL (revalidation): `just post-work WP-1-MEX-v1.2-Runtime-v2` fails phase gate (missing "SKELETON APPROVED" marker); SPEC_CURRENT mismatch (packet v02.96 vs repo v02.99); packet contains non-ASCII bytes. [READY FOR DEV]
- **[WP-1-Terminal-LAW-v2]** / FAIL (revalidation): `just post-work WP-1-Terminal-LAW-v2` fails phase gate (missing "SKELETON APPROVED" marker); SPEC_CURRENT mismatch (packet v02.96 vs repo v02.99); packet contains non-ASCII bytes. [READY FOR DEV]
- **[WP-1-Capability-SSoT]** / FAIL (revalidation): `just post-work WP-1-Capability-SSoT` fails (C701-G05 post_sha1 mismatch) for `src/backend/handshake_core/src/capabilities.rs`. [READY FOR DEV]
- **[WP-1-LLM-Core]** / FAIL (revalidation): `just post-work WP-1-LLM-Core` fails phase gate (SKELETON appears before BOOTSTRAP); packet non-ASCII + missing COR-701 manifest. [READY FOR DEV]
- **[WP-1-Flight-Recorder-v2]** / FAIL (revalidation): `node scripts/validation/gate-check.mjs WP-1-Flight-Recorder-v2` fails (missing SKELETON APPROVED marker); `node scripts/validation/post-work-check.mjs WP-1-Flight-Recorder-v2` fails (non-ASCII packet + missing COR-701 manifest). Packet references v02.96 not v02.99. [READY FOR DEV]
- **[WP-1-Tokenization-Service-20251228]** / FAIL (revalidation) (Tokenization-Service-v2): `node scripts/validation/gate-check.mjs WP-1-Tokenization-Service-20251228` fails (missing SKELETON APPROVED marker); `node scripts/validation/post-work-check.mjs WP-1-Tokenization-Service-20251228` fails (non-ASCII packet + missing COR-701 manifest). Packet references v02.96 not v02.99. [READY FOR DEV]
- **[WP-1-Flight-Recorder-UI-v2]** / FAIL (revalidation): `just gate-check WP-1-Flight-Recorder-UI-v2` fails (missing "SKELETON APPROVED" marker); `node scripts/validation/post-work-check.mjs WP-1-Flight-Recorder-UI-v2` fails (non-ASCII packet + missing COR-701 manifest). Packet references v02.93 not v02.99; user signature field missing/pending. [READY FOR DEV]
- **[WP-1-ACE-Validators-v3]** / FAIL (revalidation): `just post-work WP-1-ACE-Validators-v3` fails (non-ASCII packet + missing COR-701 manifest). Packet references v02.93 not v02.99; user signature field missing; TASK_BOARD was inconsistent with packet status history. [READY FOR DEV]
- **[WP-1-AI-Job-Model-v3]** / FAIL (revalidation): `just post-work WP-1-AI-Job-Model-v3` fails phase gate (missing "SKELETON APPROVED" marker); `node scripts/validation/post-work-check.mjs WP-1-AI-Job-Model-v3` fails (non-ASCII packet + missing COR-701 manifest). Packet references v02.93 not v02.99; packet already contains a prior FAIL section; spec updated in v02.99 to include Stalled and expanded JobKind, revalidate against new list. [READY FOR DEV]
- **[WP-1-AppState-Refactoring-v2]** / FAIL (revalidation): `just post-work WP-1-AppState-Refactoring-v2` fails phase gate (missing "SKELETON APPROVED" marker); `node scripts/validation/post-work-check.mjs WP-1-AppState-Refactoring-v2` fails (non-ASCII packet + missing COR-701 manifest). Packet references v02.93 not v02.99. [READY FOR DEV]
- **[WP-1-Flight-Recorder]** / FAIL: Spec drift vs SPEC_CURRENT v02.94; recheck A11.5 retention/telemetry. [READY FOR DEV]
- **[WP-1-ACE-Runtime]** / FAIL: Spec drift vs SPEC_CURRENT v02.94; refresh ACE-RAG-001 evidence. [READY FOR DEV]
- **[WP-1-Mutation-Traceability]** / FAIL: Spec drift vs SPEC_CURRENT v02.94; revalidate A2.9.3. [READY FOR DEV]
- **[WP-1-Dual-Backend-Tests]** / FAIL: Missing STATUS/SPEC_CURRENT; re-anchor to v02.94 A2.3.12. [READY FOR DEV]
- **[WP-1-Flight-Recorder-UI]** / FAIL (SUPERSEDED by v2): DuckDB log store + UI wiring. [READY FOR DEV]
- **[WP-1-Operator-Consoles]** - Superseded by v1 (comprehensive rewrite anchored to v02.96). [SUPERSEDED]
- **[WP-1-Metrics-OTel]** / FAIL: OpenTelemetry instrumentation. [READY FOR DEV]
- **[WP-1-Diagnostic-Pipe]** - Absorbed into WP-1-Operator-Consoles-v1 (DIAG-SCHEMA is prerequisite). [SUPERSEDED]
- **[WP-1-OSS-Governance]** / FAIL: Component Register, Copyleft isolation. [READY FOR DEV]
- **[WP-1-Supply-Chain-MEX]** / FAIL: MEX v1.2 Security Gates (gitleaks, osv-scanner). [READY FOR DEV]
- **[WP-1-ACE-Auditability]** / FAIL: ContextPlan, ContextSnapshot artifacts. [READY FOR DEV]
- **[WP-1-ACE-Validators]** / FAIL: 12 Runtime Validators (A2.6.6.7.11). [READY FOR DEV]
- **[WP-1-RAG-Iterative]** / FAIL: Snippet-first policy, search->read separation. [READY FOR DEV]
- **[WP-1-Model-Profiles]** / FAIL: ModelProfile/Routing/SafetyProfile schema. [READY FOR DEV]
- **[WP-1-MEX-Safety-Gates]** / FAIL: Guard, Container, Quota engines. [READY FOR DEV]
- **[WP-1-MEX-Observability]** / FAIL: Profiler, Monitor, Repo, Formatter engines. [READY FOR DEV]
- **[WP-1-MEX-UX-Bridges]** / FAIL: Clipboard and Notifier capability actions. [READY FOR DEV]
- **[WP-1-MCP-Skeleton-Gate]** / FAIL: MCP transport, Gate middleware. [READY FOR DEV]
- **[WP-1-AI-UX-Actions]** / FAIL: Command Palette: "Ask", "Summarize". [READY FOR DEV]
- **[WP-1-AI-UX-Rewrite]** / FAIL: Rewrite selection, structured patches, Diff view. [READY FOR DEV]
- **[WP-1-AI-UX-Summarize-Display]** / FAIL: Align with v02.84 spec; add Evidence Mapping. [READY FOR DEV]
- **[WP-1-Editor-Hardening]** / FAIL: Tiptap/Excalidraw "No Silent Edits". [READY FOR DEV]
- **[WP-1-Canvas-Typography]** / FAIL: Font Registry, offline packs, no flash. [READY FOR DEV]
- **[WP-1-PDF-Pipeline]** / FAIL: Typst + qpdf deliverable packaging. [READY FOR DEV]
- **[WP-1-Photo-Studio]** / FAIL: Skeleton surface, thumbnails, recipes. [READY FOR DEV]
- **[WP-1-Atelier-Lens]** / FAIL: Role claiming, SceneState, ConflictSet. [READY FOR DEV]
- **[WP-1-Calendar-Lens]** / FAIL: Local ActivitySpan selection UI. [READY FOR DEV]
- **[WP-1-Distillation]** / FAIL: Teacher metadata, Skill Bank schema. [READY FOR DEV]
- **[WP-1-Governance-Hooks]** / FAIL: Diary RID mapping, CI compliance. [READY FOR DEV]
- **[WP-1-Workspace-Bundle]** / FAIL: Backup/transfer export. [READY FOR DEV]
- **[WP-1-Semantic-Catalog]** / FAIL: Implement SemanticCatalog per A2.6.7. [READY FOR DEV]
- **[WP-1-MCP-End-to-End]** / FAIL: MCP capability/logging chain. [READY FOR DEV]
- **[WP-1-Metrics-Traces]** / FAIL: OTel metrics/traces + validator pack. [READY FOR DEV]


## In Progress

(Concurrency: each in-progress WP entry MUST include `ASSIGNED_TO: Coder-A` or `ASSIGNED_TO: Coder-B`; in-progress WPs must have disjoint `IN_SCOPE_PATHS` per [CX-CONC-001].)

## Done
- **[WP-1-Security-Gates-v3]** - Remediation: protocol-clean packet (ASCII + COR-701 manifest) anchored to SPEC_CURRENT v02.99; terminal security gates revalidated per A10.1 and FR-EVT-001; `just post-work WP-1-Security-Gates-v3` passes. [VALIDATED]
- **[WP-1-Gate-Check-Tool-v2]** - Hardened `scripts/validation/gate-check.mjs` to ignore prose/fenced code blocks and require explicit markers; `just post-work WP-1-Gate-Check-Tool-v2` passes. [VALIDATED]
- **[WP-1-Workflow-Engine-v4]** - Startup recovery gate enforced (HSK-WF-003 ordering) + FR-EVT-WF-RECOVERY payload aligned to SPEC_CURRENT v02.99; `just post-work WP-1-Workflow-Engine-v4` passes. [VALIDATED]
- **[WP-1-Debug-Bundle-v3]** - Remediation: Debug Bundle export conforms to SPEC_CURRENT v02.99 (10.5.6.1-12, 11.5 FR-EVT-005) and has a protocol-valid validation report. [VALIDATED]
- **[WP-1-Validator-Error-Codes-v1]** - Remediation: `just validator-error-codes` passes (stringly errors removed; nondeterminism uses require waiver markers). [VALIDATED]
- **[WP-1-Storage-Foundation-v3]** - Remediation: `src/backend/handshake_core/src/models.rs` no longer leaks `sqlx::...` types; mandatory audit passes (no `sqlx::`/`SqlitePool` references outside storage); `cargo test` and storage DAL audit pass. [VALIDATED]



## Blocked

---

## Superseded (Archive)
- **[WP-1-Storage-Foundation-20251228]** - Superseded by WP-1-Storage-Foundation-v3 (packet fails gates; spec drift; repo fails mandatory audit). [SUPERSEDED]
- **[WP-1-Gate-Check-Tool]** - Superseded by WP-1-Gate-Check-Tool-v2 (remediation: false positives blocking valid packets). [SUPERSEDED]
- **[WP-1-Operator-Consoles-v1]** - Superseded by v2 (revalidation vs v02.99; remediation in progress). [SUPERSEDED]
- **[WP-1-Operator-Consoles]** - Superseded by v1 (comprehensive rewrite anchored to v02.96 §10.5 + §11.4).
- **[WP-1-Diagnostic-Pipe]** - Absorbed into WP-1-Operator-Consoles-v1 (DIAG-SCHEMA §11.4 is prerequisite component).
- **[WP-1-Flight-Recorder]** - Superseded by v2 (Spec alignment §11.5).
- **[WP-1-Workflow-Engine-v3]** - Superseded by v4 (protocol clean packet aligned to v02.99 and HSK-WF-003 ordering).
- **[WP-1-Workflow-Engine-v2]** - Superseded by v3 (Audit remediation).
- **[WP-1-AI-Job-Model-v2]** - Superseded by v3 (Spec alignment §2.6.6.2.8).
- **[WP-1-ACE-Validators-v2]** - Superseded by v3 (Hardened security remediation).
- **[WP-1-Security-Gates]** - Superseded by v2 (Spec drift v02.84 → v02.96).
- **[WP-1-Security-Gates-v2]** - Superseded by v3 (revalidation FAIL: non-ASCII packet, missing COR-701 manifest fields, spec drift, unwrap in terminal/redaction.rs).
- **[WP-1-Terminal-LAW]** - Superseded by v2 (Stale SPEC_ANCHOR, incomplete structure).
- **[WP-1-MEX-v1.2-Runtime]** - Superseded by v2 (Stale SPEC_ANCHOR v02.84, no implementation).
- **[WP-1-Debug-Bundle-v2]** - INVALIDATED: Spec-to-code mismatches vs v02.99 (schemas/VAL-BUNDLE-001/API/FR-EVT-005) and missing protocol-valid validation report; superseded by WP-1-Debug-Bundle-v3. [SUPERSEDED]
- **[WP-1-Debug-Bundle]** - Superseded by v2 (Stale SPEC_ANCHOR v02.84, comprehensive rewrite with 10.5.6.5-12 enrichment).
