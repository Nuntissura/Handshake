# ORCHESTRATOR PRIORITIES

**Authoritative Strategic Focus for Lead Architect**
**Status:** ACTIVE
**Updated:** 2025-12-25 (Reconstructed after Stabilization)

## 1. Primary Objectives (Phase 1 Closure)

1.  **[PRIORITY_1] Storage Backend Portability:** 
    -   Enforce the four pillars defined in §2.3.12.
    -   Block all database-touching work that bypasses the `Database` trait.
    -   Goal: Make PostgreSQL migration a 1-week task.

2.  **[PRIORITY_2] Spec-to-Code Alignment:** 
    -   Enforce **[CX-598] (Main-Body Alignment)**.
    -   "Done" is 100% implementation of Main Body text, not just roadmap bullets.
    -   Reject any Work Packet that treats the Main Body as optional.

3.  **[PRIORITY_3] Deterministic Enforcement:** 
    -   Enforce the **Signature Gate [CX-585C]** and **Spec-Version Lock [CX-585A]**.
    -   Zero implementation without a prior "Strategic Pause" for technical refinement.

## 2. Risk Management Focus

-   **Anti-Vibe Guard:** Audit every Coder submission for placeholders, `unwrap()`, and generic JSON blobs.
-   **Security Gates:** Prioritize WP-1-Security-Gates to ensure MEX runtime integrity.
-   **Supply Chain Safety:** Maintain the OSS_REGISTER and block un-vetted dependencies.

## 3. Operational Directives

-   **No Role-Switching:** Orchestrators MUST NOT code.
-   **Transcription over Invention:** Task Packets must only point to the Spec (SPEC_ANCHOR). 
-   **Evidence-Based Review:** Validators MUST open files and verify line numbers.
