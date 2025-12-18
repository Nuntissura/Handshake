# ADR-0001: Handshake Architecture, Governance, and AI/Mechanical Integration

- **Status:** Accepted  
- **Date:** 2025-12-09  
- **Context:** Handshake v02.14 consolidates product vision, architecture, governance, mechanical engines, and continuous distillation. This ADR captures the foundational decisions so future changes can be evaluated without re-deriving intent from the full master spec or logger.

## Decision
- **Architecture & runtime:** Local-first desktop app using Tauri + React frontend, Rust backend orchestrator, SQLite workspace DB. One-command dev, single-user first, offline-capable.
- **Data model:** RDD split (Raw/Derived/Display) with provenance; workspace DB schemas for workspaces, documents/blocks, canvases, nodes/edges. Shadow Workspace for indexing/RAG.
- **AI control plane:** Global AI Job Model + Workflow Engine; all AI and mechanical actions are jobs flowing through Gate/Body with capability enforcement, Flight Recorder logging, and non-silent edits.
- **Governance & safety:** Capability/consent model applied to all jobs; validation/gates (Diary/COR-701) integrated; PII/secret redaction for logs; export controls for sensitive artifacts.
- **Observability:** Flight Recorder, metrics, traces required across phases; Job History/Debug panel as human-facing surfaces; provenance + trace IDs mandatory.
- **Mechanical engines:** 22 mechanical engines treated as first-class profiles (Context, Version, Sandbox, Publisher, Archivist/Librarian/Wrangler/DBA, Director/Composer, ASR, Spatial/Machinist/Simulation/Hardware/Guide). Each runs via Workflow Engine with capability gates and sidecar provenance/artifact hashes.
- **Docling & ASR:** Docling is the ingestion engine (Section 6.1); ASR integrated as a dedicated profile (Section 6.2); both reuse the AI Job/Workflow/Flight Recorder stack.
- **Continuous distillation / Skill Bank:** Canonical spec imported as Section 9; Skill Bank, checkpoints, eval/promotion are first-class. Distillation jobs run through Workflow Engine with lineage (parent_checkpoint_id), data_signature, job_ids_json, tokenizer metadata, reward features, pass@k/compile/test/collapse metrics, and promotion/rollback gates.
- **Roadmap alignment:** Phased delivery (0–4) with explicit Mechanical and Distillation tracks; every phase ships at least one mechanical slice and, from Phase 1, distillation schema/logging; promotion gates and multi-user/export governance arrive by Phase 3–4.

## Alternatives Considered
- **Direct model calls without Workflow/Jobs:** Rejected; no traceability, no gates, no governance alignment.
- **Cloud-first architecture:** Rejected; violates local-first/offline goal and privacy posture.
- **Treat mechanical/distillation as plugins only:** Rejected; required first-class integration for observability, provenance, and capability enforcement.
- **Token logging for distillation:** Rejected; text-only logging with per-engine tokenization to avoid tokenizer drift and leakage.

## Consequences
- **Pros:** Strong traceability and safety; consistent observability; reproducible actions; clear provenance; modular engines; future AI/MECH additions follow the same rails.
- **Cons:** Higher complexity/overhead; stricter gating may slow feature hacks; performance/footprint must be managed for local resources; roadmap discipline required.

## Follow-ups
- Add ADRs per major decision area (e.g., runtime/tooling, capability model shape, Shadow Workspace design, Docling choice, ASR choice, distillation eval suite).
- Link ADRs from logger entries when decisions change; update status (superseded) as the architecture evolves.