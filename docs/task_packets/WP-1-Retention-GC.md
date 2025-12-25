# Work Packet: WP-1-Retention-GC

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec §2.3.11 Retention & Pruning  
**USER_SIGNATURE:** <pending>

---

## Executive Summary
Implement retention/pruning (GC) per §2.3.11: RetentionPolicy schema, janitor engine contract, GC event logging, and pinning invariants to prevent disk bloat while preserving auditability.

**Current State:**
- No enforced retention policies or GC janitor; risk of disk bloat and missing audit logs of pruning.

**End State:**
- RetentionPolicy and PruneReport types implemented; pinning invariant enforced.
- Janitor operation (`engine.janitor` prune) implemented; GC emits Flight Recorder events.
- Tests cover policy application, pinning, and GC event logging.

**Effort:** 8-12 hours  
**Phase 1 Blocking:** YES (Spec §2.3.11)

---

## Technical Contract (LAW)
Governed by Master Spec §2.3.11:
- Implement RetentionPolicy (ArtifactKind, window_days, min_versions) and PruneReport.
- Enforce pinning invariant; emit meta.gc_summary Flight Recorder events.
- Implement engine.janitor prune operation with atomic materialize and path safety.

---

## Scope
### In Scope
1) RetentionPolicy/PruneReport types and storage.
2) Janitor prune operation; logging to Flight Recorder.
3) Tests covering pinning, pruning behavior, and event emission.

### Out of Scope
- UI for retention configuration (Phase 2).
- Advanced scheduling (Phase 2+).

---

## Quality Gate
- **RISK_TIER:** MEDIUM
- **DONE_MEANS:**
  - RetentionPolicy/PruneReport implemented per §2.3.11; pinning invariant enforced.
  - Janitor prune operation emits GC summaries to Flight Recorder.
  - Tests cover pinning and pruning behavior.
  - No forbidden patterns (unwrap/expect/panic/dbg in prod).
- **TEST_PLAN:**
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-spec-regression`
  - `just validator-hygiene-full`

---

## BOOTSTRAP
- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (§2.3.11); src/backend/handshake_core/src/diagnostics.rs; src/backend/handshake_core/src; Flight Recorder integration points
- **SEARCH_TERMS:** "RetentionPolicy", "PruneReport", "gc", "prune", "meta.gc_summary", "pinning"
- **RUN_COMMANDS:** cargo test; just validator-spec-regression; just validator-hygiene-full
- **RISK_MAP:** "Disk bloat -> instability"; "Missing GC logs -> audit gap"; "Pinning violated -> user data loss"

---

## Success Metrics
| Metric | Target | Verification |
|--------|--------|--------------|
| Pinning enforced | 100% | Tests |
| GC events logged | meta.gc_summary emitted | Tests |
| No panics | Typed errors only | Code evidence |

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>
