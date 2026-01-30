# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `docs/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `docs/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Git-Engine-Decision-Gate-v1

## STUB_METADATA
- WP_ID: WP-1-Git-Engine-Decision-Gate-v1
- BASE_WP_ID: WP-1-Git-Engine-Decision-Gate
- CREATED_AT: 2026-01-30T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: docs/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.123.md 7.6.3 (Phase 1) -> Git engine integration decision gate (git CLI external_process vs libgit2 vs go-git)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.123.md 6.3.10.1 Repo Engine (Version Control) (Mechanical) (Normative)
  - Handshake_Master_Spec_v02.123.md 11.7 OSS Register (go-git/libgit2 entries and integration posture) (Normative)

## INTENT (DRAFT)
- What: Record and enforce a single Phase 1 “Repo engine” implementation path (git CLI `external_process` vs `go-git` vs `libgit2`) and ensure the chosen posture is reflected in OSS governance + capability gating.
- Why: Prevent drift (multiple git backends), license posture mistakes, and nondeterministic repo behavior across platforms.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Decision record (ADR or equivalent) naming the single MVP path and the rationale.
  - Enforcement: code/config/registry prevents accidental use of non-selected backends.
  - Default posture: `git` CLI `external_process` unless/ until a different choice is explicitly recorded.
  - OSS Register alignment for the chosen backend(s) and explicit license posture recording.
- OUT_OF_SCOPE:
  - Implementing full repo-management UX or advanced git features (rebases, interactive staging, etc.).
  - Phase 2+ remote execution / multi-user git collaboration.

## ACCEPTANCE_CRITERIA (DRAFT)
- A single “repo engine backend” is selected and enforced; no silent fallback to another implementation.
- OSS Register contains accurate entries for any git backend dependencies used.
- Capability gating + Flight Recorder logging cover repo engine execution paths (no bypass).

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: OSS governance enforcement and mechanical tool bus / gate wiring.
- Overlaps with: Terminal LAW (process execution posture) and any repo-management job profile wiring.

## RISKS / UNKNOWNs (DRAFT)
- Risk: license posture mistakes (libgit2 is unusual); must be explicit and enforced.
- Risk: platform-specific behavior differences across backends; enforcement must be deterministic and auditable.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `docs/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `docs/refinements/WP-1-Git-Engine-Decision-Gate-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Git-Engine-Decision-Gate-v1` (in `docs/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `docs/TASK_BOARD.md` entry from STUB to Ready for Dev.

