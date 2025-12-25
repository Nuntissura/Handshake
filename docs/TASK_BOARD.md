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

---

## In Progress

(None)

## Ready for Dev

- **[WP-1-Storage-Abstraction-Layer]**

## Done

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