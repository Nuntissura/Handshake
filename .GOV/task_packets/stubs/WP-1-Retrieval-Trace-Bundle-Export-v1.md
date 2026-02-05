# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Retrieval-Trace-Bundle-Export-v1

## STUB_METADATA
- WP_ID: WP-1-Retrieval-Trace-Bundle-Export-v1
- BASE_WP_ID: WP-1-Retrieval-Trace-Bundle-Export
- CREATED_AT: 2026-01-30T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.123.md 7.6.3 (Phase 1) -> Retrieval trace bundle exporter (trace_id -> redacted-by-default evidence bundle)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.123.md 2.6.6.7.14 ACE-RAG-001 (QueryPlan + RetrievalTrace) (Normative)
  - Handshake_Master_Spec_v02.123.md 2.3.10 Artifact/Export system (redaction, exportable flags, materialize semantics) (Normative)

## INTENT (DRAFT)
- What: Provide a dedicated exporter that takes a `trace_id` (retrieval execution) and emits a redacted-by-default bundle containing QueryPlan/RetrievalTrace + budgets/cache markers + selected spans and referenced artifacts.
- Why: Make retrieval issues debuggable, explainable, and shareable with deterministic evidence without leaking sensitive payloads.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - A mechanical job (profile) to export a retrieval evidence bundle by `trace_id`.
  - Bundle contents: QueryPlan + RetrievalTrace, budgets, cache keys/markers, selected spans/truncation flags, and referenced artifacts/handles (subject to exportable policy).
  - Deterministic output structure + hashing; redacted-by-default; produces a redaction report.
  - Flight Recorder logging for exporter run + emitted artifact handles.
- OUT_OF_SCOPE:
  - Full workspace backup/export (handled by Workspace Bundle).
  - General debug bundle exporter UX beyond the retrieval-focused path.

## ACCEPTANCE_CRITERIA (DRAFT)
- Given the same `trace_id` and pins, exporter output is structurally deterministic and hash-stable.
- Export is redacted-by-default; exportable=false payloads are never leaked; redaction report is included.
- Bundle includes QueryPlan/RetrievalTrace and enough context to reproduce/explain retrieval decisions (budgets, spans, cache markers).

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: persisted QueryPlan/RetrievalTrace and artifact handle resolution; export/redaction policy enforcement.
- Coordinates with: Debug Bundle and Operator Consoles (avoid duplicated bundle formats; unify viewer links when possible).

## RISKS / UNKNOWNs (DRAFT)
- Risk: accidentally exporting local-only or sensitive data; must be default-deny and policy driven.
- Risk: nondeterministic bundle packing leads to unverifiable evidence; must canonicalize structure and record hashes.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Retrieval-Trace-Bundle-Export-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Retrieval-Trace-Bundle-Export-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.


