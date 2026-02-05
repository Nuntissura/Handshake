# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Role-Mailbox-v1

## STUB_METADATA
- WP_ID: WP-1-Role-Mailbox-v1
- BASE_WP_ID: WP-1-Role-Mailbox
- CREATED_AT: 2026-01-12T21:49:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: AÂ§7.6.3 (Phase 1) -> Spec Router + governance session log (MVP); governance kernel adoption
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - AÂ§2.6.8.10 Role Mailbox (Normative)
  - AÂ§2.6.8.8 Spec Session Log (Normative)
  - AÂ§11.5 Flight Recorder (LAW)

## INTENT (DRAFT)
- What: Implement Role Mailbox threads/messages with transcription links and always-on deterministic repo export to `.GOV/ROLE_MAILBOX/`.
- Why: Enable LLM role-to-role coordination without Operator copy/paste while preserving "chat is not state" and auditable governance.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - RoleMailbox data model + storage as artifacts.
  - Always-on export format to repo (`index.json`, `threads/*.jsonl`, `export_manifest.json`) with canonical JSON rules.
  - Flight Recorder events `FR-EVT-GOV-MAILBOX-001/002/003` and Spec Session Log entries.
  - Transcription link UX/workflow for scope/waivers/validation findings.
- OUT_OF_SCOPE:
  - Email/IMAP mail client features (10.3 Mail Client).
  - Any bypass of canonical artifact authority (messages remain non-authoritative).

## ACCEPTANCE_CRITERIA (DRAFT)
- GOV_STANDARD and GOV_STRICT maintain continuous export to `.GOV/ROLE_MAILBOX/` (no silent drift).
- Export is deterministic and idempotent for unchanged mailbox state.
- Governance-critical message types include transcription links to authoritative artifacts.
- Missing/out-of-sync export is surfaced as a blocking diagnostic (cannot silently handoff/validate).

## DEPENDENCIES / BLOCKERS (DRAFT)
- Artifact storage and hashing primitives.
- Flight Recorder event emission + schema validation.
- Spec Session Log persistence and indexing.

## RISKS / UNKNOWNs (DRAFT)
- Sensitive data leakage via exports; requires classification/redaction policy.
- Deterministic serialization across OS/runtime versions.
- Concurrency/races when multiple roles write mailbox simultaneously.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Role-Mailbox-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Role-Mailbox-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.


