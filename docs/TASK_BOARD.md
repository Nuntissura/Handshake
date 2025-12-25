# Handshake Project Task Board (Phase 1: RE-OPENED FOR SPEC ALIGNMENT)

This board provides an at-a-glance overview of all work packets (WPs) in the project. It is the single source of truth for task status and timeline. It is maintained by the Orchestrator agent.

---

## 🚨 PHASE 1 CLOSURE GATES (BLOCKING - MUST COMPLETE)

**Authority:** Master Spec §2.3.12, Architecture Decision 2025-12-25

Storage Backend Portability is foundational for Phase 1 closure. These four work packets MUST complete before Phase 1 can close.

### Storage Backend Portability Foundation (Sequential Priority)

1. **[WP-1-Storage-Abstraction-Layer]** - Define trait-based storage API, force all DB access through single module. [READY FOR DEV 🔴]
   - Lead: Coder (Senior Systems Engineer)
   - Effort: 15-20 hours
   - Blocker: None (foundational)

2. **[WP-1-AppState-Refactoring]** - Remove SqlitePool exposure from AppState, use Arc<dyn Database>. [GAP 🟡]
   - Lead: Coder (Senior Systems Engineer)
   - Effort: 8-10 hours
   - Blocker: WP-1-Storage-Abstraction-Layer (MUST COMPLETE FIRST)

3. **[WP-1-Migration-Framework]** - Rewrite migrations with portable SQL syntax, add schema versioning. [GAP 🟡]
   - Lead: Coder (Senior Systems Engineer)
   - Effort: 10-12 hours
   - Blocker: None (can start independently)

4. **[WP-1-Dual-Backend-Tests]** - Add PostgreSQL to CI, parameterize tests for both backends. [GAP 🟡]
   - Lead: DevOps/Test Engineer
   - Effort: 8-10 hours
   - Blocker: WP-1-Storage-Abstraction-Layer + WP-1-Migration-Framework

### Additional Phase 1 Must-Deliver (per Master Spec v02.84)

- **[WP-1-Tokenization-Service]** - Implement TokenizationService per §4.6 (GPT + Llama tokenizers, fallback, budgeting). [READY FOR DEV 🔴]
- **[WP-1-Semantic-Catalog]** - Implement SemanticCatalog per §2.6.7 with capability filtering. [READY FOR DEV 🔴]
- **[WP-1-Mutation-Traceability]** - Enforce No Silent Edits per §2.9.3 (StorageGuard + MutationMetadata). [READY FOR DEV 🔴]
- **[WP-1-Retention-GC]** - Implement retention/pruning per §2.3.11 (RetentionPolicy, Janitor, GC logs). [READY FOR DEV 🔴]
- **[WP-1-AppState-Refactoring]** - Remove SqlitePool exposure from AppState, use Arc<dyn Database>. [GAP 🟡]
- **[WP-1-Migration-Framework]** - Rewrite migrations with portable SQL and schema versioning. [GAP 🟡]
- **[WP-1-Dual-Backend-Tests]** - Add PostgreSQL to CI, parameterize tests for both backends. [GAP 🟡]
- **[WP-1-Security-Gates]** - Terminal/RCE guardrails (timeout/output/cwd/allowlist), secret scans. [GAP 🟡]
- **[WP-1-Operator-Consoles-v1]** - Timeline/Jobs/Problems/Evidence views for Flight Recorder/diagnostics. [GAP 🟡]
- **[WP-1-Metrics-Traces]** - Baseline metrics/OTel, validator pack. [GAP 🟡]
- **[WP-1-Capability-SSoT]** - Centralized CapabilityRegistry, single source of truth. [GAP 🟡]
- **[WP-1-MCP-End-to-End]** - Capability metadata/logging chain for MCP; end-to-end gate. [GAP 🟡]
---

## In Progress

(None)

## Ready for Validation (re-opened for review)

- **[WP-1-Terminal-Integration-Baseline]**
- **[WP-1-Capability-Enforcement]**
- **[WP-1-Flight-Recorder-UI]**
- **[WP-1-AI-UX-Summarize-Display]**
- **[WP-1-AI-Integration-Baseline]**
- **[WP-1-Frontend-AI-Action]**
- **[WP-1-Frontend-Build-Debug]**
- **[WP-1-AI-Core-Backend]**
- **[WP-Test-Sample]**
- **[WP-Codex-v0.8]**

## Ready for Dev

- **[WP-1-Storage-Abstraction-Layer]**
- **[WP-1-Tokenization-Service]**
- **[WP-1-Semantic-Catalog]**
- **[WP-1-Mutation-Traceability]**
- **[WP-1-Retention-GC]**

## Done

(None – all prior Done items re-opened for validation)
