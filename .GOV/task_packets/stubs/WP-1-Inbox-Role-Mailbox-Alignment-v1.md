# Task Packet Stub: WP-1-Inbox-Role-Mailbox-Alignment-v1

**Status:** STUB (Not Activated)

## Identity
- WP_ID: WP-1-Inbox-Role-Mailbox-Alignment-v1
- BASE_WP_ID: WP-1-Inbox-Role-Mailbox-Alignment
- Created: 2026-01-28
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (Handshake_Master_Spec_v02.120.md)

## Roadmap pointer (non-authoritative)
- Handshake_Master_Spec_v02.120.md 7.6.3 (Phase 1) -> MUST deliver (1) Model runtime integration -> [ADD v02.120] "Inbox" label alignment to Role Mailbox + runtime mailbox telemetry + Debug Bundle export coverage

## SPEC_ANCHOR_CANDIDATES (Main Body, authoritative)
- Handshake_Master_Spec_v02.120.md 2.6.8.10 Role Mailbox (Normative) (includes "Inbox" terminology clarification) [ADD v02.120]
- Handshake_Master_Spec_v02.120.md 2.6.8.10.1 Recommended Role Mailbox Body Schema (Normative) [ADD v02.120]
- Handshake_Master_Spec_v02.120.md 11.5.3.1 FR-EVT-RUNTIME-MAILBOX-101..106 (Runtime Mailbox Telemetry) (Normative) [ADD v02.120]
- Handshake_Master_Spec_v02.120.md 9.3 Debug Bundle Inclusion (Normative)
- Handshake_Master_Spec_v02.120.md 10.5 Operator Consoles: Debug & Diagnostics (Normative)

## Intent (draft)
- What: Ensure "Inbox" UI is explicitly the Role Mailbox subsystem (no parallel semantics), implement runtime mailbox telemetry events, and ensure mailbox exports/refs are included in Debug Bundles per spec.
- Why: v02.120 formalizes terminology and requires telemetry + export coverage for audit/debug.

## Scope sketch (draft)
- In scope:
  - UI label alignment ("Inbox" == Role Mailbox) and string/UX consistency.
  - Emit FR-EVT-RUNTIME-MAILBOX-* events leak-safely (no inline bodies).
  - Debug Bundle export includes required mailbox-related data/refs and any clarification thread exports per spec.
- Out of scope:
  - Making Role Mailbox authoritative for scope/requirements (explicitly forbidden by spec).

## Activation checklist (before any coding)
1. In-chat Technical Refinement Block (per .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md).
2. USER_SIGNATURE.
3. Create .GOV/refinements/WP-1-Inbox-Role-Mailbox-Alignment-v1.md.
4. Create official task packet via `just create-task-packet WP-1-Inbox-Role-Mailbox-Alignment-v1`.
5. Move Task Board entry out of STUB.


