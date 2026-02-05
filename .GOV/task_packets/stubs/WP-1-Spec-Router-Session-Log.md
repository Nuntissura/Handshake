# Work Packet Stub: WP-1-Spec-Router-Session-Log

## STUB_METADATA
- WP_ID: WP-1-Spec-Router-Session-Log
- CREATED_AT: 2026-01-07
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.101.md 7.6.3 item 18 ([ADD v02.101] Spec Router and governance session log (MVP))
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.101.md 2.6.6.6.5 Spec Router Job Profile (Normative)
  - Handshake_Master_Spec_v02.101.md 2.6.8.2 Governance Modes (Normative) (always-on Lens invariant)
  - Handshake_Master_Spec_v02.101.md 2.6.8.3 Mode Selection Policy (Normative)
  - Handshake_Master_Spec_v02.101.md 2.6.8.5 Prompt-to-Spec Router (Normative)
  - Handshake_Master_Spec_v02.101.md 2.6.8.8 Spec Session Log (Task Board + Work Packets) (Normative)
  - Handshake_Master_Spec_v02.101.md 10.5.5.9 Spec Session Log (UI)
  - Handshake_Master_Spec_v02.101.md 11.1.5 Spec Router Policy (Normative)
  - Handshake_Master_Spec_v02.101.md 11.1.6 Capability Registry Build (Normative) (capability_registry_version pinning)

## INTENT (DRAFT)
- What: Implement the Spec Router (job_kind + artifacts + policy-bound mode selection) and the Spec Session Log (Task Board + Work Packet ledger) including a UI view with deep links.
- Why: v02.101 Phase 1 requires prompt-to-spec routing that produces auditable artifacts and makes planning state visible without drifting governance.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - `spec_router` job_kind and SpecRouterJobProfile, including mode selection policy and VersionControl gating (git vs non-git).
  - Emit SpecIntent + SpecRouterDecision artifacts with `capability_registry_version` pinned.
  - Create/update Task Board + Work Packet entries for GOV_STRICT/GOV_STANDARD and append Spec Session Log entries (parallel to Flight Recorder).
  - Spec Session Log UI view (Operator Consoles): filters, timeline overlay, and deep links to SpecIntent/Decision/Binding + related Flight Recorder traces.
  - Enforce: Atelier Lens (claim + glance) always-on for ingestion + spec routing; disable only via explicit LAW override.
- OUT_OF_SCOPE:
  - Multi-user governance, remote sync, or cloud routing.
  - Phase 2+ ingestion/shadow workspace features beyond what is required for Phase 1 Spec Router flows.

## ACCEPTANCE_CRITERIA (DRAFT)
- Running a prompt-to-spec flow produces SpecIntent + SpecRouterDecision with pinned `capability_registry_version`.
- GOV_STRICT/GOV_STANDARD flows create/update Task Board and Work Packet entries and append a Spec Session Log entry.
- Git workflows require safety commit behavior only when `version_control == Git`; non-git workflows never attempt a commit.
- Spec Session Log view exists and deep-links to the relevant artifacts and Flight Recorder traces (same spec_id/work_packet_id lineage).

## DEPENDENCIES / BLOCKERS (DRAFT)
- Capability registry generation/pinning (11.1.6) must exist and be accessible to routing.
- Existing Task Board + Work Packet persistence model may require new storage/schema.
- Operator Consoles may need a new view + navigation/deep link primitives.

## RISKS / UNKNOWNs (DRAFT)
- Likely needs split into multiple WPs (backend router + ledger, UI view, LAW override wiring) to keep scope bounded.
- Interaction with existing task packet/signature workflow must not bypass repo-enforced gates.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm authoritative SPEC_ANCHOR set (Main Body sections above; not just Roadmap).
- [ ] Produce in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Spec-Router-Session-Log.md` (approved/signed).
- [ ] Create official task packet via `just create-task-packet WP-1-Spec-Router-Session-Log` (in `.GOV/task_packets/`).
- [ ] Move Task Board entry out of STUB into Ready for Dev (In Progress is handled via Validator status-sync when work begins).


