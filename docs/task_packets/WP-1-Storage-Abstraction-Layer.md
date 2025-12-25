# Work Packet: WP-1-Storage-Abstraction-Layer

**Status:** READY FOR DEV 🔴
**Authority:** Lead Architect + Validator
**Reference:** Master Spec §2.3.12, CODER_PROTOCOL "Storage Abstraction Enforcement"
**USER_SIGNATURE:** ilja251220250328

---

## Executive Summary

Establish a single, trait-based storage API that forces all database access through `src/backend/handshake_core/src/storage/` module. This WP forms the foundation for backend portability and is a Phase 1 closure gate.

**Current State:**
- Storage module exists but is incomplete
- `AppState` directly exposes `SqlitePool` and `DuckDbConnection`
- API handlers can bypass storage module with direct pool access

**End State:**
- Single trait-based `Database` interface hides all backend differences
- `AppState` exposes `Arc<dyn Database>` instead of concrete types
- ALL database access routes through storage module (enforced by pre-commit checks)

**Effort:** 15-20 hours
**Phase 1 Blocking:** YES - Must complete before Phase 1 closure

---

## Technical Contract (LAW)

This work is governed by the **Master Spec v02.84**. You MUST implement the exact trait signature and architectural pattern defined in the following sections:

- **SPEC_ANCHOR:** §2.3.12.1 (Four Portability Pillars)
- **SPEC_ANCHOR:** §2.3.12.2 (Portable SQL Examples)
- **SPEC_ANCHOR:** §2.3.12.3 (Storage API Abstraction Pattern / Database Trait Contract)
- **SPEC_ANCHOR:** §2.3.12.5 (Phase 1 Closure Requirements)

---

## Scope

### In Scope

1. **Define Storage Trait Interface** [CX-DBP-010]
   - Implement `pub trait Database` exactly as defined in §2.3.12.3.
   - Capture 100% of operations currently done with direct pool access.

2. **Implement SqliteDatabase Wrapper** [CX-DBP-010]
   - Implement `pub struct SqliteDatabase` as per §2.3.12.3.
   - Migrate all existing storage logic to trait methods.
   - Use portable SQL syntax ($1, $2) as per §2.3.12.2.

3. **Create PostgreSQL Stub** [CX-DBP-010]
   - Create `pub struct PostgresDatabase` stub as per §2.3.12.3.
   - Proves trait-based design is feasible.

4. **Refactor AppState** [CX-DBP-010]
   - Refactor `AppState` to use `Arc<dyn Database>` as per §2.3.12.3.
   - Update all handlers to use `state.storage.*` instead of `state.pool`.

5. **Audit & Enforce Boundaries** [CX-DBP-010]
   - Add pre-commit hook to reject direct pool access.
   - Ensure zero leakage of concrete types outside `storage/`.

### Out of Scope

- Rewriting migrations (→ WP-1-Migration-Framework)
- Adding PostgreSQL tests (→ WP-1-Dual-Backend-Tests)
- Changing timestamp handling strategy (→ post-foundation refinement)

---

## Quality Gate

- **RISK_TIER:** HIGH (Foundational Refactor)
- **DONE_MEANS:**
  - Passed Senior Scrutiny (clippy clean, no string-errors, boundary-safe).
  - Matches 100% of Main Body text in §2.3.12.
  - Zero direct pool access outside `storage/` (verified by grep).
  - All tests pass with the new abstraction.
- **TEST_PLAN:**
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-spec-regression`
  - `just validator-dal-audit`
  - `just validator-hygiene-full`
  - `just validator-error-codes`

---

## BOOTSTRAP

- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md; src/backend/handshake_core/src/lib.rs; src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/api/
- **SEARCH_TERMS:** "SqlitePool"; "sqlx::query"; ".pool"; "DefaultStorageGuard"
- **RUN_COMMANDS:** cargo test; just validate; grep -r "SqlitePool" src/backend/handshake_core/src/api/
- **RISK_MAP:** "Hollow trait implementation -> Portability Failure"; "Incomplete handler refactor -> Compile Error"; "Broken mutation guard -> Security Risk"

---

## Success Metrics

| Metric | Target | Verification |
|--------|--------|---|
| **Test Coverage** | 100% pass rate | `cargo test storage` |
| **API Handlers** | 0 direct pool access | Grep confirms none found |
| **Trait Methods** | 100% implemented | All methods in impl block |
| **Code Quality** | No vibe-coding | Validator anti-vibe check |
| **Validator Sign-Off** | PASS | Validator report issued |

---

**Last Updated:** 2025-12-25
**User Signature Locked:** ilja251220250328
