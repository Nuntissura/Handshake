# Task Packet: WP-1-Capability-SSoT

## Metadata
- TASK_ID: WP-1-Capability-SSoT
- DATE: 2025-12-25T20:05:00Z
- REQUESTOR: ilja
- AGENT_ID: Orchestrator
- ROLE: Orchestrator


## SKELETON APPROVED
- RISK_TIER: MEDIUM
  - Justification: Foundation for security and tool filtering; refactors core workflow logic.
- USER_SIGNATURE: ilja251220252005

---

## ??????? CODE ARCHAEOLOGY NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (??1-6, ??9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator must check `capabilities.rs`.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (??11.1 Capability Registry).
3. Surface-level compliance with roadmap bullets is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## USER_CONTEXT (Non-Technical Explainer) [CX-654]
This service acts as the project's "Security ID Office." Instead of having every part of the software guess what an AI model is allowed to do, we centralize all permissions (Capabilities) and roles (Profiles) into one authoritative registry. This ensures that an AI cannot "invent" a permission or bypass security gates by using hardcoded shortcuts.

---

## SCOPE

### Executive Summary
Implement `CapabilityRegistry` and `CapabilityProfile` models as the Single Source of Truth (SSoT) for all security and tool-filtering logic. Refactor `workflows.rs` and `api/jobs.rs` to consume this registry instead of local `Lazy<HashMap>` or hardcoded logic.

### IN_SCOPE_PATHS
- src/backend/handshake_core/src/capabilities.rs (New implementation)
- src/backend/handshake_core/src/workflows.rs (Refactor capability checks)
- src/backend/handshake_core/src/api/jobs.rs (Refactor profile mapping)
- src/backend/handshake_core/src/lib.rs (AppState wiring)
- src/backend/handshake_core/src/models.rs (Add RegistryError variants)
- src/backend/handshake_core/Cargo.toml (Dependency fix)
- src/backend/handshake_core/src/storage/retention.rs (Build fix only)
- src/backend/handshake_core/src/storage/sqlite.rs (Build fix only)

### OUT_OF_SCOPE
- UI for capability management.
- Dynamic profile persistence (Loaded from static config for now).
- Capability-scoped resource access (Phase 2).

---

## QUALITY GATE

- **TEST_PLAN**:
  ```bash
  # Core unit tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml capabilities
  
  # Integration workflow tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflow
  
  # Phase gate
  just gate-check WP-1-Capability-SSoT
  
  # Final hygiene
  just post-work WP-1-Capability-SSoT
  ```
- **DONE_MEANS**:
  - ??? `CapabilityRegistry` implemented per Master Spec ??11.1.
  - ??? All hardcoded capability maps removed from `workflows.rs`.
  - ??? `CapabilityProfile` objects (e.g. 'Analyst', 'Coder') are whitelists of Registry IDs.
  - ??? Integration tests verify unknown Capability ID results in deterministic error.
  - ??? Evidence mapping block is complete.

- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-hash>
  # Manual steps:
  # 1. Remove src/backend/handshake_core/src/capabilities.rs
  # 2. Revert AppState in lib.rs
  # 3. Restore hardcoded maps in workflows.rs
  ```

---

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md
  * Handshake_Master_Spec_v02.84.md (??11.1)
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/src/api/jobs.rs
  * src/backend/handshake_core/src/lib.rs

- **SEARCH_TERMS**:
  * "CapabilityRegistry"
  * "capability_profile"
  * "term.exec"
  * "doc.summarize"
  * "enforce_capabilities"

- **RUN_COMMANDS**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gate-check WP-1-Capability-SSoT
  ```

- **RISK_MAP**:
  * "Incomplete refactor" -> Runtime panics
  * "Registry drift" -> Security bypass
  * "Type mismatch" -> Compilation failure

---

## EVIDENCE_MAPPING
- CapabilityRegistry SSoT (valid axes/full IDs, profiles, job/job_profile maps, HSK-4001 errors) -> src/backend/handshake_core/src/capabilities.rs:25-213
- Axis inheritance + profile whitelist enforcement in workflows with Flight Recorder log -> src/backend/handshake_core/src/workflows.rs:100-145
- API uses registry for job_kind ??? capability_profile_id mapping -> src/backend/handshake_core/src/api/jobs.rs:57-73
- Unknown capability/profile errors surface via registry -> src/backend/handshake_core/src/workflows.rs:285-350
- sqlx offline cache regenerated for storage (compilation unblock) -> src/backend/handshake_core/.sqlx/

---

## VALIDATION
- cargo sqlx prepare -- --bin handshake_core --lib --tests (DATABASE_URL=sqlite:///D:/Projects/LLM projects/Handshake/src/backend/handshake_core/.sqlx/dev.db) -> PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml capabilities (DATABASE_URL=sqlite:///D:/Projects/LLM projects/Handshake/src/backend/handshake_core/.sqlx/dev.db) -> PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflow (DATABASE_URL=sqlite:///D:/Projects/LLM projects/Handshake/src/backend/handshake_core/.sqlx/dev.db) -> PASS
- just gate-check WP-1-Capability-SSoT -> PASS
- just post-work WP-1-Capability-SSoT -> PASS

---

## AUTHORITY
- **SPEC_ANCHOR**: ??11.1 (Capability Registry)
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: docs/TASK_BOARD.md

---

**Last Updated:** 2025-12-25
**User Signature Locked:** ilja251220252005

## VALIDATION REPORT ??? WP-1-Capability-SSoT
  Verdict: PASS ???

  The Work Packet WP-1-Capability-SSoT has been successfully implemented, verified, and closed.

  Summary of Work:
   1. Capability Registry (SSoT): Implemented a centralized registry in src/backend/handshake_core/src/capabilities.rs that defines
      valid capabilities, profiles, and job requirements.
   2. Workflow Integration: Refactored src/backend/handshake_core/src/workflows.rs to replace hardcoded permission maps with
      registry lookups.
   3. Safety: Capability checks now emit structured Flight Recorder events (capability_check) with "allowed" or "denied" outcomes.
   4. Hygiene: Resolved blocking build issues in the storage module (unrelated but necessary for validation).
   5. Documentation: Updated the Task Board and appended the Validation Report to the Task Packet.

  Artifacts:
   - src/backend/handshake_core/src/capabilities.rs: New module (SSoT).
   - docs/task_packets/WP-1-Capability-SSoT.md: Closed with PASS verdict.

  Note: The Coder correctly handled the scope expansion to fix the build, ensuring the repo remains in a compilable state.
