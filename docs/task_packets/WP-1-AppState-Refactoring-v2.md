# Task Packet: WP-1-AppState-Refactoring-v2

## Metadata
- TASK_ID: WP-1-AppState-Refactoring-v2
- WP_ID: WP-1-AppState-Refactoring-v2
- DATE: 2025-12-27T00:10:00Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator
- STATUS: Ready-for-Dev

## User Context
We are cleaning up the internal wiring of the app to make it more professional and "portable." Currently, parts of the app's "engine" are directly visible to the rest of the system (like showing the internal SQLite and DuckDB machinery). This task hides those internal details behind a clean "interface," making it much easier to swap parts (like moving to a larger database) in the future without breaking everything.

## Scope
- **What**: Refactor AppState and the Database trait to enforce Trait Purity [CX-DBP-040].
- **Why**: Remediate Strategic Audit failure in WP-1-AppState-Refactoring. Direct pool exposure (`fr_pool`, `sqlite_pool`) violates the One Storage API boundary mandate of §2.3.12.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/lib.rs (AppState refactor)
  * src/backend/handshake_core/src/storage/mod.rs (Database trait cleanup)
  * src/backend/handshake_core/src/storage/retention.rs (Janitor trait usage)
  * src/backend/handshake_core/src/main.rs (init_* wiring)
  * src/backend/handshake_core/src/api/ (Ensuring no direct pool usage remains)
- **OUT_OF_SCOPE**:
  * Migration rewrites (handled in WP-1-Migration-Framework).
  * Implementation of new Database methods (focus on refactoring existing ones).

## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Core architectural change affecting AppState and all database interactions.
- **TEST_PLAN**:
  ```bash
  # 1. Compile check
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  
  # 2. Run all tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  
  # 3. Verify Trait Purity (DAL Audit)
  # - Ensure 'SqlitePool' or 'DuckDb' do not appear in src/api/
  # - Ensure 'AppState' does not contain 'fr_pool'
  
  # 4. Final hygiene
  just cargo-clean  # cleans external cargo target (../Cargo Target/handshake-cargo-target) before self-eval/commit
  just post-work WP-1-AppState-Refactoring-v2
  ```
- **DONE_MEANS**:
  * ✅ `AppState` exposes only `Arc<dyn Database>` and `Arc<dyn FlightRecorder>`.
  * ✅ `Database` trait no longer exposes `sqlite_pool()`.
  * ✅ `Janitor` refactored to use `Arc<dyn Database>` and `Arc<dyn FlightRecorder>` instead of concrete pools.
  * ✅ `main.rs` initialization returns trait objects (`Arc<dyn ...>`).
  * ✅ All API handlers consume only trait-based APIs.

## ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## BOOTSTRAP
- **FILES_TO_OPEN**:
  * src/backend/handshake_core/src/lib.rs
  * src/backend/handshake_core/src/storage/mod.rs
  * src/backend/handshake_core/src/storage/retention.rs
  * src/backend/handshake_core/src/main.rs
  * docs/SPEC_CURRENT.md (v02.93 §2.3.12.3)
- **SEARCH_TERMS**:
  * "pub struct AppState"
  * "fn sqlite_pool"
  * "fr_pool"
  * "Janitor"
- **RUN_COMMANDS**:
  ```bash
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "API Handler Breakage" -> Handlers previously relying on leaked pools (Fix: implement required logic in traits)
  * "Circular Dependency" -> Moving types between mod.rs and submodules (Fix: keep pure interfaces in mod.rs)

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md (Master Spec v02.93)
- **SPEC_ANCHOR**: §2.3.12.3 [CX-DBP-040], §2.3.11.2 [HSK-GC-005]
- **Strategic Audit Reference**: WP-1-AppState-Refactoring (Failed v1)

---

**Last Updated:** 2025-12-27
**User Signature Locked:** <pending>
