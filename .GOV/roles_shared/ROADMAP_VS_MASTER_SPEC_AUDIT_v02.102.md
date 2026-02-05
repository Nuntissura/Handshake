# Roadmap vs Master Spec Audit (v02.102)

## Scope
- Baseline spec scanned: Handshake_Master_Spec_v02.101.md (full file, top to bottom)
- Target spec produced: Handshake_Master_Spec_v02.102.md
- Roadmap location: Handshake_Master_Spec_v02.102.md Section 7.6 (Development Roadmap)
- Governance artifacts reconciled:
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/task_packets/stubs/*.md

## Deterministic method
1) Extracted the Roadmap section bounds (start: "## 7.6 Development Roadmap", end: "# 8. Reference").
2) Scanned every non-Roadmap line for WP IDs (`WP-<n>-...`) and compared the set against the Roadmap text.
3) Scanned the Phase 1 "Implementation Notes: Phase 1 Final Gaps" section (11.10) for MUST-level closure details and checked for Roadmap presence of key literals.
4) Reconciled missing Roadmap items against .GOV/roles_shared/TASK_BOARD.md and created/queued stub WPs where no packet existed.

## Findings (v02.101 -> Roadmap gaps)
### A) WP IDs referenced by Main Body but absent from Roadmap
The following WP IDs were referenced outside the Roadmap section in v02.101, but did not appear in the Roadmap:
- WP-1-Storage-Abstraction-Layer
- WP-1-AppState-Refactoring
- WP-1-Migration-Framework
- WP-1-Dual-Backend-Tests
- WP-1-Capability-SSoT
- WP-1-Global-Silent-Edit-Guard

### B) Phase 1 final-gap closure details missing as literals in Roadmap
The following Phase 1 closure literals were absent from the Roadmap in v02.101:
- HANDSHAKE_TYPST_BIN / HANDSHAKE_QPDF_BIN
- {APP_DATA}/fonts/
- http://localhost:11434/api/tags

## Actions applied in v02.102
### Master Spec / Roadmap
- Added a v02.102 Roadmap note for additive entries.
- Added Phase 1 MUST-deliver Roadmap items:
  - (CX-DBP-030) Phase 1 closure storage portability work packets list
  - CapabilityRegistry SSoT pointer (WP-1-Capability-SSoT)
  - Global Silent Edit Guard pointer (WP-1-Global-Silent-Edit-Guard)
  - Phase 1 final-gap closure literals (Section 11.10): typst/qpdf discovery, fonts runtime root, Ollama startup detection

### Task Board and stubs
- .GOV/roles_shared/TASK_BOARD.md:
  - Added Ready for Dev entries for WP-1-Storage-Abstraction-Layer and WP-1-Migration-Framework (previously missing from board).
  - Added Stub Backlog entry for WP-1-Global-Silent-Edit-Guard.
- .GOV/task_packets/stubs/WP-1-Global-Silent-Edit-Guard.md:
  - Created as a stub packet with Main Body SPEC_ANCHOR candidates.

## Open issues / conflicts to resolve with Operator
1) Phase 1 closure work packets are referenced in Main Body (Section 2.3.12.5 [CX-DBP-030]) using base WP IDs, while .GOV/roles_shared/TASK_BOARD.md often tracks v2/v3 packets.
   - Decision needed: whether to explicitly map "base WP" -> "current superseding WP-vN" in governance docs, or to keep both tracked.
2) v02.102 approval recorded: USER_SIGNATURE `ilja080120262305` logged in `.GOV/roles_shared/SIGNATURE_AUDIT.md` and applied to `.GOV/roles_shared/SPEC_CURRENT.md` + spec changelog.

