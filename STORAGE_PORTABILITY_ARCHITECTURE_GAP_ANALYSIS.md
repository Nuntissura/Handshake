# Storage Backend Portability: Architecture Gap Analysis & Governance Plan

**Date:** 2025-12-25
**Authority:** Architecture Review (Phase 1 Alignment)
**Status:** GOVERNANCE DECISION REQUIRED
**Requestor:** User (Mid-Phase 1 strategic concern)

---

## Executive Summary

**The Problem:** Handshake currently uses SQLite locally (for local-first design) but the codebase has **no enforced abstraction layer** that would make migration to PostgreSQL/cloud backends feasible without significant rewrites. The four best practices for keeping "Postgres later cheap" are NOT implemented.

**The Gap:** Four critical architectural gaps exist:
1. ❌ **No Single Storage API** - Database access leaks into API endpoints
2. ❌ **SQLite-Specific SQL Syntax** - Migrations and queries use `?1`, `strftime()`, triggers (non-portable)
3. ❌ **No Portable Schema** - Migrations use SQLite dialect exclusively
4. ❌ **No Dual-Backend Tests** - No test infrastructure for PostgreSQL compatibility

**The Opportunity:** Mid-Phase 1 is the IDEAL time to establish these patterns (cost is low now; cost is exponential in Phase 2+).

**The Decision Required:** These requirements must live in **THREE places simultaneously**:
- **Master Spec Main Body** — Architectural requirement (LAW)
- **CODER_PROTOCOL** — Implementation enforcement (HOW)
- **Task Board / Work Packets** — Implementation tracking (WHEN)

---

## Part 1: Current State Assessment

### 1.1 What Exists (Inventory)

**Storage Module:**
- File: `src/backend/handshake_core/src/storage/mod.rs` (284 lines)
- Contains: `StorageGuard` trait, `DefaultStorageGuard` implementation
- Functions: `replace_blocks_with_guard()`, `update_block_raw_content()`
- Tests: 3 unit tests (sqlite::memory: only)

**AppState Exposure:**
- File: `src/backend/handshake_core/src/lib.rs` (lines 38-39)
- Direct exposure: `pub pool: SqlitePool`, `pub fr_pool: Arc<Mutex<DuckDbConnection>>`
- **Impact:** Any module can directly access databases, bypassing storage module

**Migrations:**
- Count: 6 migrations (0001_init.sql → 0006_calendar_law.sql)
- Format: Raw SQL files (not managed by migration framework)
- Dialect: Pure SQLite syntax
- Examples:
  - Line 14 (0001): `strftime('%Y-%m-%d %H:%M:%f', 'now')` ← SQLite-only
  - Line 18-23 (0002): TRIGGER syntax ← SQLite-specific
  - Line 27: `DEFAULT (strftime(...))` ← SQLite datetime function

**Master Spec References:**
- §2.3.5.6 "The Role of SQLite" (line 1988)
  - States SQLite is for "indexing", not primary storage
  - Does NOT mention portability or multi-backend support
- §2.3.5.2 discusses DuckDB + graph extensions (line 2220)
- §3.3.0.2 "SQLite: The Recommended Choice" (line 9416)
  - Recommends SQLite for local-first
  - Does NOT mandate abstraction for future migration

**CODER_PROTOCOL:**
- No explicit constraints on database access patterns
- No prohibition against direct `sqlx::query()`
- No requirement for portable SQL syntax

---

### 1.2 Gap Analysis: Four Best Practices vs. Current Implementation

| Best Practice | Required | Currently Implemented | Gap | Impact |
|---|---|---|---|---|
| **One Storage API** | ✅ CRITICAL | ⚠️ PARTIAL (module exists, but bypassed) | Exposed SqlitePool in AppState | **HIGH** - Any code can do raw DB access |
| **Portable Schema/Migrations** | ✅ CRITICAL | ❌ NO (SQLite syntax only) | `strftime()`, triggers, `?1` placeholders | **HIGH** - PostgreSQL migration would require rewrite |
| **Treat Indexes as Rebuildable** | ✅ CRITICAL | ⚠️ UNKNOWN (not analyzed) | No explicit policy documented | **MEDIUM** - Need to verify |
| **Dual-Backend Tests** | ✅ CRITICAL | ❌ NO (SQLite only) | No PostgreSQL test variant | **HIGH** - No safety net for migration validation |

---

## Part 2: Specific Technical Violations

### 2.1 Non-Portable SQL Examples (Evidence)

**File:** `src/backend/handshake_core/src/storage/mod.rs`

**Issue 1: SQLite Placeholder Syntax**
```rust
// Line 91-94
sqlx::query(
    r#"
    SELECT COUNT(1) FROM ai_jobs WHERE id = ?1
    "#,
)
```
**Problem:** `?1` is SQLite-specific. PostgreSQL uses `$1`.
**Portability Cost:** Every query must be rewritten.

**Issue 2: SQLite DateTime Functions**
```sql
-- File: src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql, Line 14
created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
```
**Problem:** `strftime()` is SQLite-specific. PostgreSQL uses `NOW()` or `CURRENT_TIMESTAMP`.
**Portability Cost:** Schema must be rewritten.

**Issue 3: SQLite Triggers (Issue 3: SQLite Triggers**
```sql
-- File: src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql, Line 18-23
CREATE TRIGGER ai_jobs_updated_at
AFTER UPDATE ON ai_jobs
FOR EACH ROW
BEGIN
    UPDATE ai_jobs SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = OLD.id;
END;
```
**Problem:** Syntax differs between SQLite and PostgreSQL (no OLD/NEW auto-updates in same way).
**Portability Cost:** Triggers must be rewritten; alternative approaches (application-layer timestamps) needed.

**Issue 4: Direct SqlitePool Exposure**
```rust
// File: src/backend/handshake_core/src/lib.rs, Line 38
pub pool: SqlitePool,
```
**Problem:** API endpoints can access database directly, bypass storage guards.
**Example:** Any handler can do `state.pool.execute()` without going through storage module.
**Portability Cost:** Can't swap backends without checking ALL handlers.

---

### 2.2 Storage Module Bypass Examples

**Current Pattern (WRONG):**
```rust
// Hypothetical API endpoint
#[tauri::command]
async fn get_blocks(state: AppState, doc_id: String) -> Result<Vec<Block>> {
    // BYPASS 1: Direct pool access
    let blocks = sqlx::query_as::<_, Block>(
        "SELECT * FROM blocks WHERE document_id = ?"
    )
    .bind(&doc_id)
    .fetch_all(&state.pool)  // ← Direct access, bypasses storage module
    .await?;

    Ok(blocks)
}
```

**What It Should Be:**
```rust
#[tauri::command]
async fn get_blocks(state: AppState, doc_id: String) -> Result<Vec<Block>> {
    // Use storage module
    let blocks = state.storage.get_blocks(&doc_id).await?;
    Ok(blocks)
}
```

---

## Part 3: Where This Lives (Governance Locations)

### 3.1 Master Spec Main Body (New Section)

**Required Addition:** New §2.3.X "Storage Backend Portability Architecture"

**Purpose:** Establish that Handshake MUST support multiple backends (not just SQLite).

**Placement:** After §2.3.5 (Database & Sync Patterns), before §2.4 (Extraction Pipeline).

**Content Outline:**
```
§2.3.6 Storage Backend Portability Architecture

1. Architectural Requirement (MUST)
   - Handshake supports local-first (SQLite) and cloud-optional (PostgreSQL)
   - Single Storage API enforces all database access through module boundary

2. Four Portability Pillars (MUST)
   - One Storage API: state.storage.* (no state.pool direct access)
   - Portable Schema: DB-agnostic SQL, version-managed migrations
   - Rebuildable Indexes: Prefer recompute over row migration
   - Dual-Backend Testing: SQLite + PostgreSQL in CI

3. Technical Constraints
   - No SQLite-specific SQL syntax (no ?1 placeholders, no strftime())
   - Migrations use parameterized DDL (Liquibase/Flyway compatible)
   - Application layer handles timestamps (not DB triggers)

4. Rationale
   - Phase 1 cost: 2-3 WPs, ensures decoupling early
   - Phase 2+ cost: Rewrite entire storage layer if not done now
   - Alignment: Local-first philosophy requires flexibility for future backends
```

---

### 3.2 CODER_PROTOCOL (New Constraints)

**Required Addition:** New section "Storage Abstraction Enforcement"

**Content:**

```markdown
### Storage Abstraction Enforcement [CX-XXX]

#### Rule 1: Single Storage API
- FORBIDDEN: Direct access to `state.pool` or `state.fr_pool`
- FORBIDDEN: Raw `sqlx::query()` outside `src/storage/` module
- REQUIRED: All DB operations through `state.storage.*` interface

#### Rule 2: Portable SQL Syntax
- FORBIDDEN: SQLite placeholders (?1, ?2) → USE: $1, $2 (portable)
- FORBIDDEN: strftime(), sqlite functions → USE: parameterized timestamps
- FORBIDDEN: SQLite triggers → USE: application-layer mutation tracking

#### Rule 3: Migration Versioning
- REQUIRED: Numbered migration files (0001_, 0002_, ...)
- REQUIRED: Schema-only mutations (no data transforms in schema migrations)
- REQUIRED: Migrations tested against both SQLite and PostgreSQL

#### Rule 4: Type Skeleton for Storage Modules
- REQUIRED: Every new storage function MUST declare portable types
- Example:
  ```rust
  pub async fn get_blocks(
      db: &Database,  // ← Abstract, not SqlitePool
      doc_id: &str,
  ) -> Result<Vec<Block>>
  ```
- NO: `pub async fn get_blocks(pool: &SqlitePool, ...)`

#### Enforcement
- Pre-commit validation: Grep for `sqlx::query` outside storage/ → FAIL
- Pre-commit validation: Grep for `state.pool`, `state.fr_pool` in API handlers → FAIL
- CI: Run dual-backend tests (SQLite + Postgres)
```

---

### 3.3 Task Board & Work Packets (Implementation Tracking)

**Required Work Packets (Phase 1):**

| WP ID | Title | Purpose | Dependencies |
|-------|-------|---------|--------------|
| **WP-1-Storage-Abstraction-Layer** | Formalize storage module boundaries | Force all DB access through `state.storage.*` | None |
| **WP-1-Migration-Framework** | Adopt migration versioning (sqlx::migrate compatible) | Ensure schema portability | Storage-Abstraction |
| **WP-1-Dual-Backend-Tests** | Add PostgreSQL test variant | Validate portability as code evolves | Migration-Framework |
| **WP-1-AppState-Refactoring** | Remove direct pool exposure | Hide `SqlitePool` and `DuckDbConnection` | Storage-Abstraction |

**Blocking Constraints:**
- Phase 1 CANNOT close without completing all four WPs
- New features that touch database MUST use portable SQL
- Any PR that violates CX-XXX rules will be BLOCKED by validator

---

## Part 4: Implementation Plan

### 4.1 Phase 1 Execution (Immediate)

**Step 1: Spec Enrichment** (1-2 hours)
- [ ] Add §2.3.6 to Master Spec Main Body
- [ ] Version spec to v02.82
- [ ] Cross-reference new RID tags (CX-XXX)

**Step 2: Protocol Enforcement** (1-2 hours)
- [ ] Add storage constraints to CODER_PROTOCOL
- [ ] Add validator checks for DAL boundaries
- [ ] Add migration pattern verification to VALIDATOR_PROTOCOL

**Step 3: Work Packet Creation** (2-3 hours)
- [ ] Create WP-1-Storage-Abstraction-Layer task packet
- [ ] Create WP-1-Migration-Framework task packet
- [ ] Create WP-1-Dual-Backend-Tests task packet
- [ ] Create WP-1-AppState-Refactoring task packet
- [ ] Add blocking constraints to Task Board

**Step 4: Immediate Code Enforcement** (1 hour)
- [ ] Add pre-commit hook to reject direct pool access in API handlers
- [ ] Add CI validation for portable SQL syntax

### 4.2 Priority Order

```
MUST DO FIRST (Blocking):
1. Storage-Abstraction-Layer (defines module boundary)
2. AppState-Refactoring (hides raw pools)

CAN FOLLOW:
3. Migration-Framework (makes existing migrations portable)
4. Dual-Backend-Tests (validates ongoing changes)
```

---

## Part 5: Risk Assessment

### 5.1 If We DO This Now (Phase 1)

**Effort:**
- Spec enrichment: 2 hours
- Protocol enforcement: 2 hours
- Work packets: 3 hours
- Code refactoring: 10-15 hours (spread across 4 WPs)
- **Total: ~20 hours (doable in Phase 1)**

**Benefit:**
- PostgreSQL migration later = 1-2 weeks (low friction)
- No existential rework needed in Phase 2+
- Architectural clarity enforced through code

**Risk:**
- None. All changes are deferred until WPs are created.
- Spec/Protocol changes are governance, not code.

---

### 5.2 If We DON'T Do This Now

**Cost in Phase 2:**
- Every single query must be rewritten
- Every test must be duplicated (SQLite + PostgreSQL)
- AppState refactoring becomes "break everything" moment
- Risk of losing traceability when migrations are rewritten
- Estimated cost: 4-6 weeks of rework

**Risk:**
- Architectural debt compounds
- Team morale hits when Phase 2 becomes "rewrite backend"
- Spec integrity violated (if Spec says "portability supported", but code doesn't, validator catches inconsistency)

---

## Part 6: Recommendation

### 6.1 Immediate Action (TODAY)

1. **Update Master Spec Main Body** with §2.3.6
   - Status: GOVERNANCE (not code)
   - Authority: Lead Architect
   - Timeline: 2 hours

2. **Update CODER_PROTOCOL** with storage constraints
   - Status: GOVERNANCE (not code)
   - Authority: Lead Architect + Validator
   - Timeline: 2 hours

3. **Update VALIDATOR_PROTOCOL** with DAL audits
   - Status: GOVERNANCE (not code)
   - Authority: Validator
   - Timeline: 1 hour

4. **Create Work Packets** for Phase 1 Task Board
   - Status: PLANNING (not code)
   - Authority: Orchestrator
   - Timeline: 3 hours

### 6.2 Phase 1 Execution (This Sprint)

Treat the four WPs as **Phase 1 closure gates**:
- **WP-1-Storage-Abstraction-Layer** (Blocking for all other storage WPs)
- **WP-1-AppState-Refactoring** (Blocks removal of direct pool access)
- **WP-1-Migration-Framework** (Makes existing migrations portable)
- **WP-1-Dual-Backend-Tests** (Validates ongoing changes)

---

## Part 7: Answers to User's Specific Questions

### Q: "Is this Master Spec related?"
**A:** YES. §2.3.6 must be added to Main Body to establish "backend portability" as an architectural requirement.

### Q: "Do we have to make additions to the main body?"
**A:** YES. Portability must be in Main Body because:
- It's an architectural constraint (not implementation detail)
- It affects Phase 2+ feasibility
- Validator audits Main Body for completeness
- [CX-598] Main-Body Alignment Invariant demands it

### Q: "Update the roadmap?"
**A:** YES. Update §7.6 (Development Roadmap) to flag "Storage backend portability" as Phase 1 closure gate.

### Q: "Update taskboard and workpackets?"
**A:** YES. Create 4 new Work Packets (all Phase 1):
- WP-1-Storage-Abstraction-Layer
- WP-1-Migration-Framework
- WP-1-Dual-Backend-Tests
- WP-1-AppState-Refactoring

### Q: "Enforced through coder protocol?"
**A:** YES. CODER_PROTOCOL gets new section "Storage Abstraction Enforcement" with specific rules (no direct pool access, portable SQL syntax, etc.).

### Q: "Or all of the above?"
**A:** **ALL OF THE ABOVE.** This is a complete governance system:
- **Spec** = LAW (what we're building)
- **Protocol** = HOW (how coders implement it)
- **Task Board** = WHEN (when we'll do it)

---

## Part 8: Governance Approval Checklist

- [ ] **Lead Architect** reviews and approves §2.3.6 addition to Spec
- [ ] **Validator** reviews and approves CODER_PROTOCOL + VALIDATOR_PROTOCOL updates
- [ ] **Orchestrator** creates 4 Work Packets and adds Phase 1 blocking constraints
- [ ] **All Protocols** cross-referenced with new RIDs (CX-XXX)
- [ ] **Task Board** updated with Phase 1 gate: "Storage Backend Portability Complete"

---

**Status:** AWAITING GOVERNANCE APPROVAL

*This document provides complete architectural clarity on where backend portability lives in the governance system. User is empowered to make decision on Spec/Protocol/Task Board updates.*

