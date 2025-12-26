# Task Packet: WP-1-ACE-Validators-v2

## Metadata
- TASK_ID: WP-1-ACE-Validators-v2
- DATE: 2025-12-26
- REQUESTOR: User
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator
- STATUS: DONE [VALIDATED]
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja261220250201

## Scope
- **What**: Implement the 8 remaining ACE Security Guards per §2.6.6.7.11.1–8.
- **Why**: Complete the mandatory security layer for the ACE runtime to ensure auditable, deterministic, and safe execution across local and cloud tiers.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/ace/validators/mod.rs
  * src/backend/handshake_core/src/ace/validators/determinism.rs
  * src/backend/handshake_core/src/ace/validators/artifact.rs
  * src/backend/handshake_core/src/ace/validators/compaction.rs
  * src/backend/handshake_core/src/ace/validators/promotion.rs
  * src/backend/handshake_core/src/ace/validators/leakage.rs
  * src/backend/handshake_core/src/ace/validators/injection.rs
  * src/backend/handshake_core/src/ace/validators/boundary.rs
  * src/backend/handshake_core/src/ace/validators/payload.rs
  * src/backend/handshake_core/src/ace/mod.rs
- **OUT_OF_SCOPE**:
  * Implementing the 4 guards already completed in WP-1-ACE-Runtime (Budget, Freshness, Drift, Cache).
  * External tool integration (Phase 2).

## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Security-critical runtime enforcement; failure allows prompt injection or data leakage.
- **TEST_PLAN**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just validator-spec-regression
  just validator-scan WP-1-ACE-Validators-v2
  just validator-hygiene-full
  ```
- **DONE_MEANS**:
  * ✅ `ContextDeterminismGuard`, `ArtifactHandleOnlyGuard`, `CompactionSchemaGuard`, `MemoryPromotionGuard`, `CloudLeakageGuard`, `PromptInjectionGuard`, `JobBoundaryRoutingGuard`, and `LocalPayloadGuard` implemented per §2.6.6.7.11 in v02.89 exactly.
  * ✅ All guards integrated into the `AceRuntimeValidator` trait pipeline.
  * ✅ Conformance tests `T-ACE-VAL-001` through `008` implemented and passing.
  * ✅ No forbidden patterns (unwrap/expect/panic/dbg/Value in domain).

## ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## BOOTSTRAP
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md
  * Handshake_Master_Spec_v02.89.md
  * src/backend/handshake_core/src/ace/mod.rs
  * src/backend/handshake_core/src/ace/validators/mod.rs
- **SEARCH_TERMS**:
  * "AceRuntimeValidator"
  * "ContextSnapshot"
  * "tool_delta_inline_char_limit"
  * "PromptInjectionGuard"
- **RUN_COMMANDS**:
  ```bash
  just dev
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Injection detection failure" -> Safety breach
  * "False positive blocking" -> UX degradation
  * "Trait signature mismatch" -> Compilation failure

## Authority
- **SPEC_ANCHOR**: §2.6.6.7.11 (ACE Security Guards)
- **SPEC_CURRENT**: Handshake_Master_Spec_v02.89.md
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: docs/TASK_BOARD.md

## Notes
- **Assumptions**: None.
- **Open Questions**: None.
- **Dependencies**: foundational.

## VALIDATION

### Implementation Status
| Guard | File | Status | Conformance Test |
|-------|------|--------|------------------|
| ContextDeterminismGuard | determinism.rs | ✅ Complete | T-ACE-VAL-001 |
| ArtifactHandleOnlyGuard | artifact.rs | ✅ Complete | T-ACE-VAL-002 |
| CompactionSchemaGuard | compaction.rs | ✅ Complete | T-ACE-VAL-003 |
| MemoryPromotionGuard | promotion.rs | ✅ Complete | T-ACE-VAL-004 |
| CloudLeakageGuard | leakage.rs | ✅ Complete | T-ACE-VAL-005 |
| PromptInjectionGuard | injection.rs | ✅ Complete | T-ACE-VAL-006 |
| JobBoundaryRoutingGuard | boundary.rs | ✅ Complete | T-ACE-VAL-007 |
| LocalPayloadGuard | payload.rs | ✅ Complete | T-ACE-VAL-008 |

### AceError Variants Added
- ACE-009: DeterminismViolation
- ACE-010: InlineDeltaExceeded
- ACE-011: CompactionSchemaViolation
- ACE-012: MemoryPromotionBlocked
- ACE-013: CloudLeakageBlocked
- ACE-014: PromptInjectionDetected
- ACE-015: JobBoundaryViolation
- ACE-016: LocalPayloadViolation

### ValidatorPipeline
- Updated `with_default_guards()` to include all 12 guards
- Updated re-exports in ace/mod.rs to include all 12 guards

### Validation Commands
```bash
just validator-scan  # PASS - no forbidden patterns
just post-work WP-1-ACE-Validators-v2  # Pending - requires VALIDATION section
```

### Known Blockers (Pre-existing)
- Build fails due to pre-existing issues in other modules (jobs.rs, storage/postgres.rs, workflows.rs)
- These issues are NOT from this work packet and require separate remediation
- ACE validators module is structurally complete and ready for testing once blockers are resolved

### Mandates Verified
- ✅ All 8 guards implement `AceRuntimeValidator` trait
- ✅ `ValidatorPipeline::with_default_guards()` includes all 12 guards
- ✅ Every validator failure emits specific `AceError` variant
- ✅ `PromptInjectionGuard` returns `AceError::PromptInjectionDetected` (triggers JobState::Poisoned)
- ✅ `CloudLeakageGuard` defaults to Block for unknown sensitivity

---

## HISTORY

### VALIDATION REPORT — WP-1-ACE-Validators-v2 (2025-12-26)
Verdict: PASS

**REASON FOR PASS:**
With the build restored by GPT Codex, the ACE validators tests now execute and pass (61 tests). The logic fulfills the normative requirements of §2.6.6.7.11 for the remaining 8 security guards. The pipeline is now complete with all 12 mandatory validators.

---

**Last Updated:** 2025-12-26
**User Signature Locked:** ilja261220250201
